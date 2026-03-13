from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_032(CardScript):
    """EX11-032 GrandGalemon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Hand][Main] If you have [Shoto Kazama], by placing 1 [Galemon] from your trash
        # as any of your [Pteromon]'s bottom digivolution card, it digivolves into this card
        # for a digivolution cost of 3, ignoring digivolution requirements.
        effect0 = ICardEffect()
        effect0._is_hand_main = True
        effect0.set_effect_name("EX11-032 [Hand][Main] Place Galemon, digivolve onto Pteromon")
        effect0.set_effect_description(
            "[Hand] [Main] If you have [Shoto Kazama], by placing 1 [Galemon] from your trash "
            "as any of your [Pteromon]'s bottom digivolution card, it digivolves into this card "
            "for a digivolution cost of 3, ignoring digivolution requirements.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card.permanent_of_this_card() is not None:
                return False  # must be in hand
            player = card.owner
            if not player or not player.is_my_turn:
                return False
            # Must have Shoto Kazama tamer on field
            has_shoto = any(
                p.contains_card_name('Shoto Kazama')
                for p in player.battle_area
            )
            if not has_shoto:
                return False
            # Must have Galemon in trash
            has_galemon_in_trash = any(
                any('Galemon' in (n or '') for n in (getattr(c, 'card_names', []) or []))
                for c in player.trash_cards
            )
            if not has_galemon_in_trash:
                return False
            # Must have Pteromon on field
            has_pteromon = any(
                p.contains_card_name('Pteromon')
                for p in player.battle_area
            )
            if not has_pteromon:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            game = ctx.get('game')
            player = ctx.get('player')
            hand_card = ctx.get('card')
            if not (game and player and hand_card):
                return

            def _is_galemon(c):
                names = getattr(c, 'card_names', []) or []
                return any('Galemon' in (n or '') for n in names)

            # Let agent choose which Galemon from trash
            def on_galemon_selected(trash_idx):
                if trash_idx >= len(player.trash_cards):
                    return
                galemon = player.trash_cards[trash_idx]

                # Select a Pteromon on field to digivolve onto
                def on_pteromon_selected(target_perm):
                    # Place Galemon from trash as bottom digi card
                    if galemon in player.trash_cards:
                        player.trash_cards.remove(galemon)
                    target_perm.add_card_source_bottom(galemon)
                    game.logger.log(
                        f"[Hand][Main] Placed {game._card_ref(galemon)} "
                        f"under {game._perm_ref(target_perm)}")

                    # Remove GrandGalemon from hand and digivolve
                    if hand_card in player.hand_cards:
                        player.hand_cards.remove(hand_card)
                    target_perm.add_card_source(hand_card)
                    target_perm.turn_digivolved = game.turn_count
                    player.lose_memory(3)
                    game.logger.log(
                        f"[Hand][Main] Digivolved {game._card_ref(hand_card)} "
                        f"onto {game._perm_ref(target_perm)} (cost: 3)")
                    player.draw()
                    game.execute_effects(EffectTiming.WhenDigivolving,
                                         {"digivolved_permanent": target_perm})

                game.effect_select_own_permanent(
                    player, on_pteromon_selected,
                    filter_fn=lambda p: p.contains_card_name('Pteromon'),
                    is_optional=False,
                    prompt="Select a [Pteromon] to digivolve into GrandGalemon.")

            # Build valid trash indices for Galemon cards
            _SEL_TRASH_START = 130
            valid_trash = []
            for i, c in enumerate(player.trash_cards):
                if _is_galemon(c):
                    valid_trash.append(_SEL_TRASH_START + i)
            if not valid_trash:
                return

            def _on_trash_action(action_id):
                idx = action_id - _SEL_TRASH_START
                on_galemon_selected(idx)

            game.request_selection(
                GamePhase.SelectTrash, player, _on_trash_action,
                valid_trash, is_optional=False,
                prompt="Select a [Galemon] from your trash to place under [Pteromon].")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
