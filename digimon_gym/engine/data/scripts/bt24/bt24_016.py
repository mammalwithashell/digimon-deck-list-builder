from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_016(CardScript):
    """BT24-016 Lamiamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Hand][Main] If you have [Owen Dreadnought], by placing 1 [Dimetromon]
        # from your trash as any of your [Elizamon]'s bottom digivolution card,
        # it digivolves into this card for a cost of 3, ignoring requirements.
        effect0 = ICardEffect()
        effect0._is_hand_main = True
        effect0.set_effect_name("BT24-016 [Hand][Main] Place Dimetromon, digivolve onto Elizamon")
        effect0.set_effect_description(
            "[Hand] [Main] If you have [Owen Dreadnought], by placing 1 "
            "[Dimetromon] from your trash as any of your [Elizamon]'s bottom "
            "digivolution card, it digivolves into this card for a digivolution "
            "cost of 3, ignoring digivolution requirements.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card.permanent_of_this_card() is not None:
                return False  # must be in hand
            player = card.owner
            if not player or not player.is_my_turn:
                return False
            # Must have Owen Dreadnought tamer on field
            has_owen = any(
                p.contains_card_name('Owen Dreadnought')
                for p in player.battle_area
            )
            if not has_owen:
                return False
            # Must have Dimetromon in trash
            has_dimetromon_in_trash = any(
                any('Dimetromon' in (n or '') for n in (getattr(c, 'card_names', []) or []))
                for c in player.trash_cards
            )
            if not has_dimetromon_in_trash:
                return False
            # Must have Elizamon on field
            has_elizamon = any(
                p.contains_card_name('Elizamon')
                for p in player.battle_area
            )
            if not has_elizamon:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            game = ctx.get('game')
            player = ctx.get('player')
            hand_card = ctx.get('card')
            if not (game and player and hand_card):
                return

            def _is_dimetromon(c):
                names = getattr(c, 'card_names', []) or []
                return any('Dimetromon' in (n or '') for n in names)

            # Let agent choose which Dimetromon from trash
            def on_dimetromon_selected(trash_idx):
                if trash_idx >= len(player.trash_cards):
                    return
                dimetromon = player.trash_cards[trash_idx]

                # Select an Elizamon on field to digivolve onto
                def on_elizamon_selected(target_perm):
                    # Place Dimetromon from trash as bottom digi card
                    if dimetromon in player.trash_cards:
                        player.trash_cards.remove(dimetromon)
                    target_perm.add_card_source_bottom(dimetromon)
                    game.logger.log(
                        f"[Hand][Main] Placed {game._card_ref(dimetromon)} "
                        f"under {game._perm_ref(target_perm)}")

                    # Remove Lamiamon from hand and digivolve
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
                    player, on_elizamon_selected,
                    filter_fn=lambda p: p.contains_card_name('Elizamon'),
                    is_optional=False,
                    prompt="Select an [Elizamon] to digivolve into Lamiamon.")

            # Build valid trash indices for Dimetromon cards
            _SEL_TRASH_START = 130
            valid_trash = []
            for i, c in enumerate(player.trash_cards):
                if _is_dimetromon(c):
                    valid_trash.append(_SEL_TRASH_START + i)
            if not valid_trash:
                return

            def _on_trash_action(idx):
                # _decode_trash_selection already subtracts SEL_TRASH_START
                on_dimetromon_selected(idx)

            game.request_selection(
                GamePhase.SelectTrash, player, _on_trash_action,
                valid_trash, is_optional=False,
                prompt="Select a [Dimetromon] from your trash to place under [Elizamon].")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Shared process for the security-manipulation effect (used by both WhenDigivolving and OnUseAttack)
        def _security_effect_process(ctx: Dict[str, Any]):
            """Opponent places 1 card from hand as bottom security, then trash their top security."""
            player = ctx.get('player')
            game = ctx.get('game')
            enemy = player.enemy if player else None
            if not (enemy and game):
                return
            if not enemy.hand_cards:
                # If opponent has no hand cards, still trash top security if able
                if enemy.security_cards:
                    trashed = enemy.security_cards.pop(0)
                    enemy.trash_cards.append(trashed)
                return

            def on_hand_selected(selected_card):
                if selected_card in enemy.hand_cards:
                    enemy.hand_cards.remove(selected_card)
                enemy.security_cards.append(selected_card)  # bottom of security
                # Trash opponent's top security card
                if enemy.security_cards:
                    trashed = enemy.security_cards.pop(0)
                    enemy.trash_cards.append(trashed)

            game.effect_select_hand_card(
                enemy, filter_fn=lambda c: True, callback=on_hand_selected,
                is_optional=False,
                prompt="Place 1 card from your hand as the bottom card of your security stack.")

        # [When Digivolving] [Once Per Turn] Opponent places 1 card from hand as bottom security. Trash their top security.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT24-016 [When Digivolving] Opponent places 1 card from hand in security bottom. Trash their security top")
        effect1.set_effect_description("[When Digivolving] [Once Per Turn] Your opponent places 1 card from their hand as the bottom security card. Then, trash their top security card.")
        effect1.set_hash_string("WAWD_BT24-016")
        effect1.is_when_digivolving = True
        effect1.set_max_count_per_turn(1)

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_security_effect_process)
        effects.append(effect1)

        # [When Attacking] [Once Per Turn] Opponent places 1 card from hand as bottom security. Trash their top security.
        effect1b = ICardEffect()
        effect1b.set_timing(EffectTiming.OnUseAttack)
        effect1b.set_effect_name("BT24-016 [When Attacking] Opponent places 1 card from hand in security bottom. Trash their security top")
        effect1b.set_effect_description("[When Attacking] [Once Per Turn] Your opponent places 1 card from their hand as the bottom security card. Then, trash their top security card.")
        effect1b.set_hash_string("WAWD_BT24-016")
        effect1b.is_on_attack = True
        effect1b.set_max_count_per_turn(1)

        def condition1b(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1b.set_can_use_condition(condition1b)
        effect1b.set_on_process_callback(_security_effect_process)
        effects.append(effect1b)

        # Timing: EffectTiming.OnLoseSecurity
        # [ESS] [Your Turn] Play 1 [Reptile] or [Dragonkin] Digimon from hand for free.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnLoseSecurity)
        effect3.set_effect_name("BT24-016 Play 1 [Reptile] or [Dragonkin] from hand")
        effect3.set_effect_description("Play Card")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("PlayDigimon_BT24_016")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only fires on your turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Reptile' in _t or 'Dragonkin' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                # Must be 5000 DP or lower
                card_dp = getattr(c, 'dp', None)
                if card_dp is None or card_dp > 5000:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
