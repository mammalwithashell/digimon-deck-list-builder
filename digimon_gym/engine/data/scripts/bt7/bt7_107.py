from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT7_107(CardScript):
    """BT7-107 Calling From the Darkness | Option Purple Cost 1

    [Main] Delete 1 of your Digimon. Then, return up to 2 purple Digimon
        cards from your trash to your hand.

    [Security] Add this card to its owner's hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Delete own Digimon, return up to 2 purple from trash ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT7-107 Delete own Digimon, return purple from trash")
        effect0.set_effect_description(
            "[Main] Delete 1 of your Digimon. Then, return up to 2 purple "
            "Digimon cards from your trash to your hand."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Delete 1 of your own Digimon (mandatory)
            own_digimon = [p for p in player.battle_area if p.is_digimon]
            if not own_digimon:
                return

            def on_delete(target_perm):
                if target_perm is None:
                    return
                player.delete_permanent(target_perm)
                # Step 2: Return up to 2 purple Digimon cards from trash to hand
                _select_purple_from_trash(player, game, 2)

            game.effect_select_own_permanent(
                player, on_delete,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Delete 1 of your Digimon."
            )

        def _select_purple_from_trash(player, game, remaining):
            """Select up to `remaining` purple Digimon from trash to hand."""
            if remaining <= 0:
                return

            from ....game.constants import SEL_TRASH_START

            def purple_digimon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return CardColor.Purple in colors

            valid_indices = []
            for i, c in enumerate(player.trash_cards):
                if purple_digimon_filter(c):
                    valid_indices.append(SEL_TRASH_START + i)
            if not valid_indices:
                return

            def on_trash_selected(action_id):
                idx = action_id - SEL_TRASH_START
                if 0 <= idx < len(player.trash_cards):
                    chosen = player.trash_cards[idx]
                    player.trash_cards.remove(chosen)
                    player.hand_cards.append(chosen)
                    # Recurse for the next selection
                    _select_purple_from_trash(player, game, remaining - 1)

            game.request_selection(
                GamePhase.SelectTrash, player, on_trash_selected,
                valid_indices, is_optional=True,
                prompt="Select a purple Digimon card from trash to add to hand.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Add this card to its owner's hand ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT7-107 Security: Add to hand")
        effect1.set_effect_description("[Security] Add this card to its owner's hand.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            if card in player.trash_cards:
                player.trash_cards.remove(card)
                player.hand_cards.append(card)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
