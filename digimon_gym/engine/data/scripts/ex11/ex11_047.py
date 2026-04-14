from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_047(CardScript):
    """EX11-047 Impmon | Lv.3

    Alt digivolve: from [Yaamon] for cost 0
    [Start of Your Main Phase] Trash 1 card in your hand. Then, gain 1 memory.
    Inherited: [Your Turn] This Digimon gets +2000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from [Yaamon] for cost 0 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-047 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Yaamon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Yaamon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Start of Your Main Phase] Trash 1, then gain 1 memory ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("EX11-047 Trash 1, gain 1 memory")
        effect1.set_effect_description(
            "[Start of Your Main Phase] Trash 1 card in your hand. Then, gain 1 memory.")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
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

            # Step 1: Trash 1 card from hand (mandatory)
            # C# reference: discard selection is inside if (HandCards.Count >= 1)
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)

            if player.hand_cards:
                game.effect_select_hand_card(
                    player, lambda c: True, on_trashed,
                    is_optional=False,
                    prompt="Trash 1 card from your hand.")

            # Step 2: Gain 1 memory (unconditional — C# AddMemory is outside
            # the hand-count check; "Then" is sequential, not conditional)
            player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited [Your Turn] +2000 DP ---
        effect2 = ICardEffect()
        effect2.set_effect_name("EX11-047 DP modifier")
        effect2.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 2000

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
