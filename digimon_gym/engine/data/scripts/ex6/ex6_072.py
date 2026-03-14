from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_072(CardScript):
    """EX6-072 Mega Digimon Assembly! | Option (White, Cost 3)

    While your opponent has a level 6 or higher Digimon, you may ignore this
    card's color requirements.
    [Main] 1 of your level 6 Digimon and 1 card in the hand may DNA digivolve
    into a level 7 Digimon card in your hand.
    [Security] Return 1 level 6 or higher Digimon card from your trash to the
    hand. Then, add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Ignore Color Req (while opponent has Lv.6+) ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-072 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req while opponent has Lv.6+ Digimon")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass  # Color requirement bypass handled at card level
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Blast DNA Digivolve ---
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-072 Blast DNA Digivolve")
        effect1.set_effect_description("Blast DNA Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_dna_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- [Main] DNA digivolve: 1 Lv.6 Digimon + 1 hand card -> Lv.7 from hand ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OptionSkill)
        effect2.set_effect_name("EX6-072 DNA Digivolve")
        effect2.set_effect_description(
            "[Main] 1 of your level 6 Digimon and 1 card in the hand may "
            "DNA digivolve into a level 7 Digimon card in your hand.")

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """2 of your Digimon may DNA digivolve into a Lv.7 Digimon from hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def dna_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                lv = getattr(c, 'level', None)
                if lv is None or lv < 7:
                    return False
                return True

            game.effect_dna_digivolve_from_hand(player, dna_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- [Security] Return 1 Lv.6+ Digimon from trash to hand. Then add this card to hand. ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("EX6-072 Return 1 Digimon from trash to hand then add this card to hand")
        effect3.set_effect_description(
            "[Security] Return 1 level 6 or higher Digimon card from your "
            "trash to the hand. Then, add this card to the hand.")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Return 1 Lv.6+ Digimon from trash to hand, then add this card to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....game.constants import SEL_TRASH_START
            from ....data.enums import GamePhase

            def trash_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                lv = getattr(c, 'level', None)
                if lv is None or lv < 6:
                    return False
                return True

            # Build valid trash indices
            valid_trash = []
            for i, c in enumerate(player.trash_cards):
                if trash_filter(c):
                    valid_trash.append(SEL_TRASH_START + i)

            if valid_trash:
                def on_trash_selected(action_id: int):
                    idx = action_id - SEL_TRASH_START
                    if 0 <= idx < len(player.trash_cards):
                        chosen = player.trash_cards[idx]
                        player.trash_cards.remove(chosen)
                        player.hand_cards.append(chosen)
                    # Then add this card to hand
                    _add_self_to_hand()

                game.request_selection(
                    GamePhase.SelectTrash, player, on_trash_selected,
                    valid_trash, is_optional=True,
                    prompt="Select a Lv.6+ Digimon from your trash to return to hand.")
            else:
                _add_self_to_hand()

            def _add_self_to_hand():
                if card and card in player.trash_cards:
                    player.trash_cards.remove(card)
                    player.hand_cards.append(card)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
