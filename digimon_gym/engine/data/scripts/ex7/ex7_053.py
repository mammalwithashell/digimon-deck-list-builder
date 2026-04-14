from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_053(CardScript):
    """EX7-053 Eyesmon: Scatter Mode | Lv.4 Purple

    [On Play] Trash 1 card in your hand. Then, you may return 1 Digimon
        card with the [Evil], [Dark Dragon] or [Evil Dragon] trait from
        your trash to the hand.
    Inherited: <Retaliation>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Trash 1, return 1 trait Digimon from trash ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX7-053 Trash 1 from hand, return trait Digimon from trash")
        effect0.set_effect_description(
            "[On Play] Trash 1 card in your hand. Then, you may return 1 Digimon "
            "card with the [Evil], [Dark Dragon] or [Evil Dragon] trait from your "
            "trash to the hand.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Trash 1 card from hand (mandatory)
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)

                # Step 2: May return 1 qualifying Digimon from trash to hand
                def trait_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    traits = getattr(c, 'card_traits', []) or []
                    qualifying = {'Evil', 'Dark Dragon', 'Evil Dragon'}
                    return any(t in qualifying for t in traits)

                qualifying = [c for c in player.trash_cards if trait_filter(c)]
                if not qualifying:
                    return

                from ....game.constants import SEL_TRASH_START
                from ....data.enums import GamePhase

                valid = [SEL_TRASH_START + i
                         for i, c in enumerate(player.trash_cards)
                         if trait_filter(c)]
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
                    GamePhase.SelectTarget, player, on_select, valid,
                    is_optional=True,
                    prompt="Return 1 [Evil]/[Dark Dragon]/[Evil Dragon] Digimon from trash to hand.")

            if player.hand_cards:
                game.effect_select_hand_card(
                    player, lambda c: True, on_trashed,
                    is_optional=False,
                    prompt="Trash 1 card from your hand.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Inherited <Retaliation> ---
        effect1 = ICardEffect()
        effect1.set_effect_name("EX7-053 Retaliation")
        effect1.set_effect_description("<Retaliation>")
        effect1.is_inherited_effect = True
        effect1._is_retaliation = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
