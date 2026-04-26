from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST10_15(CardScript):
    """ST10-15 Darkness Wave | Option Purple Cost 1

    [Main] Trash the top 3 cards of your deck. Then, if you have a yellow
        Digimon in play, return 1 yellow or purple Digimon card from your
        trash to your hand.
    [Security] Activate this card's [Main] effect.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _main_effect_logic(ctx: Dict[str, Any]):
            """Shared logic for Main and Security effects."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Trash the top 3 cards of your deck
            mill_count = min(3, len(player.library_cards))
            if mill_count > 0:
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

            # Step 2: If you have a yellow Digimon in play, return 1 yellow or
            # purple Digimon card from your trash to your hand
            has_yellow_digimon = any(
                p.is_digimon and CardColor.Yellow in (
                    getattr(p.top_card, 'card_colors', []) or []
                )
                for p in player.battle_area
                if p.top_card
            )

            if not has_yellow_digimon:
                return

            def trash_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return CardColor.Yellow in colors or CardColor.Purple in colors

            qualifying = [c for c in player.trash_cards if trash_filter(c)]
            if not qualifying:
                return

            from ....game.constants import SEL_TRASH_START
            from ....data.enums import GamePhase

            valid = [SEL_TRASH_START + i
                     for i, c in enumerate(player.trash_cards)
                     if trash_filter(c)]
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
                is_optional=False,
                prompt="Return 1 yellow or purple Digimon from trash to hand.")

        # --- Effect 0: [Main] ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("ST10-15 Trash 3, return yellow/purple Digimon")
        effect0.set_effect_description(
            "[Main] Trash the top 3 cards of your deck. Then, if you have a "
            "yellow Digimon in play, return 1 yellow or purple Digimon card "
            "from your trash to your hand."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_main_effect_logic)
        effects.append(effect0)

        # --- Effect 1: [Security] Activate [Main] effect ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("ST10-15 Security: Activate [Main] effect")
        effect1.set_effect_description("[Security] Activate this card's [Main] effect.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_main_effect_logic)
        effects.append(effect1)

        return effects
