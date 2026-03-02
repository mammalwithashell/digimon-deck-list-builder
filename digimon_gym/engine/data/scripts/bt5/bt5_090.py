from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_090(CardScript):
    """BT5-090 Arata Sanada | Tamer | Black | Cost 3

    [Start of Your Turn] If a Digimon card with the [Unidentified] trait
        is in your trash, gain 1 memory.
    [Your Turn] When one of your Digimon digivolves into [Diaboromon],
        you may suspend this Tamer to play 1 [Diaboromon] Token without
        paying the cost. (Digimon/Cost 14/Lv.6/White/Mega/Unknown/
        Unidentified/3000 DP)
    Security Effect: [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Gain 1 memory ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT5-090 Start of turn: Gain 1 memory")
        effect0.set_effect_description(
            "[Start of Your Turn] If a Digimon card with the [Unidentified] "
            "trait is in your trash, gain 1 memory."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check if there's an Unidentified Digimon in trash
            player = card.owner
            for tc in player.trash_cards:
                if tc.is_digimon and 'Unidentified' in (tc.card_traits or []):
                    return True
            return False
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Gain 1 memory."""
            game = ctx.get('game')
            if game:
                game.memory += 1
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn] On digivolve into Diaboromon, play token ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT5-090 Play Diaboromon Token on digivolve")
        effect1.set_effect_description(
            "[Your Turn] When one of your Digimon digivolves into [Diaboromon], "
            "you may suspend this Tamer to play 1 [Diaboromon] Token without "
            "paying the cost."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check this tamer is not already suspended
            own_perm = card.permanent_of_this_card() if card else None
            if own_perm and own_perm.is_suspended:
                return False
            # Check if the trigger is a digivolution into Diaboromon
            trigger_perm = context.get('permanent')
            if trigger_perm and trigger_perm.top_card and trigger_perm.top_card.c_entity_base:
                name = trigger_perm.top_card.c_entity_base.card_name_eng or ''
                if 'Diaboromon' in name:
                    # Check it's a digivolution (has more than 1 card source)
                    if len(trigger_perm.card_sources) > 1:
                        return True
            return False
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Suspend this Tamer, play a Diaboromon Token."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            own_perm = card.permanent_of_this_card() if card else None
            if own_perm:
                own_perm.suspend()
                game.effect_play_token(player, 'diaboromon')
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT5-090 Security Effect")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play this card from security without paying the cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                player.play_card_from_source(card, pay_cost=False)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
