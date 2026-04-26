from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_030(CardScript):
    """LM-030 Green Scramble | Option (Green, Cost 2)

    [Main] 1 of your green Digimon may digivolve into a green Digimon card in
        the hand with the digivolution cost reduced by 3. Then, place this card
        in the battle area.
    [Start of Your Turn] <Delay> (by trashing this card from the battle area,
        activate):
        - Return 1 green Digimon card from your trash to the top of the deck.
          Then, if you don't have a Digimon, you may play 1 green Digimon card
          with 2000 DP or less from your trash without paying the cost.
    [Security] You may play 1 green Digimon card with 2000 DP or less from
        your trash without paying the cost. Then, add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Green Digimon digivolves with cost -3 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name(
            "LM-030 Digivolve 1 green Digimon with cost reduced by 3")
        effect0.set_effect_description(
            "[Main] 1 of your green Digimon may digivolve into a green "
            "Digimon card in the hand with the digivolution cost reduced by 3. "
            "Then, place this card in the battle area."
        )
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Select a green Digimon on field; digivolve into green from hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def hand_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return any(col.name == 'Green' for col in colors)

            def own_filter(p):
                if not p.is_digimon:
                    return False
                top = p.top_card
                if top is None:
                    return False
                colors = getattr(top, 'card_colors', []) or []
                if not any(col.name == 'Green' for col in colors):
                    return False
                return any(hand_filter(c) for c in player.hand_cards)

            def on_select(target_perm):
                game.effect_digivolve_from_hand(
                    player, target_perm, hand_filter,
                    cost_reduction=3, is_optional=True)

            game.effect_select_own_permanent(
                player, on_select, filter_fn=own_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Delay marker ---
        effect1 = ICardEffect()
        effect1.set_effect_name("LM-030 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Fires at start of owner's turn.
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [Start of Your Turn] Delay — return green Digimon from
        # trash to deck top; if no Digimon, play green Digimon ≤ 2000 DP ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartTurn)
        effect2.set_effect_name(
            "LM-030 Delay: Return green Digimon from trash to deck top; "
            "if no Digimon, play green Digimon <=2000 DP from trash")
        effect2.set_effect_description(
            "[Start of Your Turn] <Delay> — "
            "Return 1 green Digimon card from your trash to the top of the "
            "deck. Then, if you don't have a Digimon, you may play 1 green "
            "Digimon card with 2000 DP or less from your trash without paying "
            "the cost."
        )
        effect2.is_optional = True
        effect2._is_delay_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Note: _execute_delay trashes the card BEFORE calling this condition,
            # so permanent_of_this_card() would return None. Don't check it here.
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # C# requires opponent to have at least 1 Digimon
            enemy = card.owner.enemy if card and card.owner else None
            if not enemy:
                return False
            return any(p.is_digimon for p in enemy.battle_area)

        effect2.set_can_use_condition(condition2)

        # Trash selection action-ID offset (mirrors game.py SEL_TRASH_START)
        _SEL_TRASH_START = 130

        def process2(ctx: Dict[str, Any]):
            """Return green Digimon to deck top; play if no field Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Note: the engine's _execute_delay already trashes this card
            # from the battle area before calling this callback.

            def _green_digimon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return any(col.name == 'Green' for col in colors)

            # Return 1 green Digimon from trash to the top of the deck (player selection).
            valid_trash = []
            for i, c in enumerate(player.trash_cards):
                if _green_digimon_filter(c):
                    valid_trash.append(_SEL_TRASH_START + i)

            def _after_return():
                # Then, if you don't have a Digimon on field, play green <= 2000 DP.
                has_digimon = any(
                    p.is_digimon for p in (player.battle_area or [])
                )
                if not has_digimon:
                    def play_filter(c):
                        if not getattr(c, 'is_digimon', False):
                            return False
                        colors = getattr(c, 'card_colors', []) or []
                        if not any(col.name == 'Green' for col in colors):
                            return False
                        dp = getattr(c, 'base_dp', None)
                        if dp is not None and dp > 2000:
                            return False
                        return True

                    game.effect_play_from_zone(
                        player, 'trash', play_filter,
                        free=True, is_optional=True)

            if valid_trash:
                def on_trash_selected(action_id: int):
                    idx = action_id - _SEL_TRASH_START
                    if 0 <= idx < len(player.trash_cards):
                        moved = player.trash_cards.pop(idx)
                        player.library_cards.insert(0, moved)
                    _after_return()

                game.request_selection(
                    GamePhase.SelectTrash,
                    player,
                    on_trash_selected,
                    valid_trash,
                    is_optional=False,
                    prompt="Select a green Digimon from your trash to place at the top of your deck.",
                )
            else:
                _after_return()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] Play green Digimon ≤ 2000 DP from trash;
        # then add this card to hand ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name(
            "LM-030 Security: Play green Digimon <=2000 DP from trash, "
            "add this card to hand")
        effect3.set_effect_description(
            "[Security] You may play 1 green Digimon card with 2000 DP or "
            "less from your trash without paying the cost. Then, add this card "
            "to the hand."
        )
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Play green Digimon ≤ 2000 DP from trash; add this card to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                if not any(col.name == 'Green' for col in colors):
                    return False
                dp = getattr(c, 'base_dp', None)
                if dp is not None and dp > 2000:
                    return False
                return True

            game.effect_play_from_zone(
                player, 'trash', play_filter,
                free=True, is_optional=True)

            # Add this card to hand. The engine trashes the security card
            # before the security effect fires; pop the last trashed card
            # (which is this card) back to hand.
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
