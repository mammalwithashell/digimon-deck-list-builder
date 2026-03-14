from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_005(CardScript):
    """EX11-005 Yaamon | Digi-Egg

    Inherited: [Start of Your Main Phase] This Digimon may digivolve into
        a [Dark Dragon] or [Evil Dragon] trait Digimon card in the trash
        with the digivolution cost reduced by 1. If this effect digivolve,
        trash 2 cards in your hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Inherited [Start of Your Main Phase] ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX11-005 Digivolve from trash, then trash 2 from hand")
        effect0.set_effect_description(
            "[Start of Your Main Phase] This Digimon may digivolve into a "
            "[Dark Dragon] or [Evil Dragon] trait Digimon card in the trash "
            "with the digivolution cost reduced by 1. If this effect digivolve, "
            "trash 2 cards in your hand.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Digivolve this Digimon into a [Dark Dragon]/[Evil Dragon] trait
            Digimon card from the trash with cost reduced by 1. If it digivolved,
            trash 2 cards from hand."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def trash_digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Dark Dragon' in t or 'Evil Dragon' in t for t in traits)

            # Find qualifying cards in trash
            candidates = [c for c in player.trash_cards if trash_digi_filter(c)]
            if not candidates:
                return

            # Use SEL_TRASH_START to let agent select from trash
            from ....game.constants import SEL_TRASH_START
            from ....data.enums import GamePhase

            valid = []
            for i, c in enumerate(player.trash_cards):
                if trash_digi_filter(c):
                    valid.append(SEL_TRASH_START + i)
            if not valid:
                return

            def on_select(action_id: int):
                idx = action_id - SEL_TRASH_START
                if not (0 <= idx < len(player.trash_cards)):
                    return
                chosen = player.trash_cards[idx]

                # Calculate cost: evo cost reduced by 1
                base_cost = getattr(chosen, 'get_cost_itself', 0)
                if callable(base_cost):
                    base_cost = base_cost
                else:
                    base_cost = base_cost
                cost = max(0, base_cost - 1)

                # Move from trash and digivolve
                player.trash_cards.remove(chosen)
                perm.add_card_source(chosen)
                perm.turn_digivolved = game.turn_count
                player.lose_memory(cost)
                game.logger.log(
                    f"[Effect Digivolve] {game._card_ref(chosen)} onto "
                    f"{game._perm_ref(perm)} (cost: {cost}, from trash)")
                player.draw()
                game.execute_effects(
                    EffectTiming.WhenDigivolving,
                    {"digivolved_permanent": perm})

                # If digivolve succeeded, trash 2 cards from hand
                _trash_count = [0]

                def _trash_one():
                    if _trash_count[0] >= 2 or not player.hand_cards:
                        return

                    def on_hand_trashed(selected):
                        if selected in player.hand_cards:
                            player.hand_cards.remove(selected)
                            player.trash_cards.append(selected)
                        _trash_count[0] += 1
                        _trash_one()

                    game.effect_select_hand_card(
                        player, lambda c: True, on_hand_trashed,
                        is_optional=False,
                        prompt=f"Trash card {_trash_count[0]+1}/2 from your hand.")

                _trash_one()

            game.request_selection(
                GamePhase.SelectTarget, player, on_select, valid,
                is_optional=True,
                prompt="Select a [Dark Dragon]/[Evil Dragon] Digimon from trash to digivolve into.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
