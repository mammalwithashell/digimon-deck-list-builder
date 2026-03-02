from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX2_046(CardScript):
    """EX2-046 ADR-02 Searcher | Digimon White | Intel Acquisition Agent"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Play cost reduction by 2 if no other ADR-02 Searcher in play ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX2-046 Reduce play cost by 2")
        effect0.set_effect_description(
            "When you would play this card from your hand, reduce its play "
            "cost by 2 if you don't have another [ADR-02 Searcher] in play."
        )
        effect0.cost_reduction = 2

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.owner:
                # Check that no other ADR-02 Searcher is in play
                has_other = any(
                    p.contains_card_name('ADR-02 Searcher')
                    for p in card.owner.battle_area
                    if p.top_card and p.top_card is not card
                )
                return not has_other
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn] This Digimon can't attack players ---
        # NOTE: Attack restriction (can't attack player) is partially modeled.
        # The engine checks _is_cannot_attack but distinguishing player-only
        # restriction is not directly supported. Marked PARTIAL.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX2-046 Can't attack players")
        effect1.set_effect_description(
            "[Your Turn] This Digimon can't attack players."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and card.owner.is_my_turn:
                return True
            return False
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [On Play] <Draw 1> ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX2-046 On Play: Draw 1")
        effect2.set_effect_description("[On Play] <Draw 1>")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Inherited [Your Turn] All D-Reaper Digimon +1000 DP ---
        effect3 = ICardEffect()
        effect3.set_effect_name("EX2-046 Inherited: D-Reaper +1000 DP")
        effect3.set_effect_description(
            "Inherited: [Your Turn] All of your Digimon with the [D-Reaper] "
            "trait get +1000 DP."
        )
        effect3.is_inherited_effect = True
        effect3.dp_modifier = 1000
        effect3._applies_to_all_own_digimon = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.owner and card.owner.is_my_turn:
                return True
            return False
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
