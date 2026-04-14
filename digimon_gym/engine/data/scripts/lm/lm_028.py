from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_028(CardScript):
    """LM-028 Blue Scramble | Option (Blue, Cost 2)

    [Main] 1 of your blue Digimon may digivolve into a blue Digimon card
        in the hand with the digivolution cost reduced by 3. Then, place this
        card in the battle area.

    [Start of Your Turn] If your opponent has a Digimon, <Delay>.
        Return 1 blue Digimon card from your trash to the top of the deck.
        Then, if you don't have a Digimon, you may play 1 blue Digimon card
        with 2000 DP or less from your trash without paying the cost.

    [Security] You may play 1 blue Digimon card with 2000 DP or less from
        your trash without paying the cost. Then, add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_blue_digimon(c) -> bool:
            if not getattr(c, 'is_digimon', False):
                return False
            colors = getattr(c, 'card_colors', []) or []
            color_names = [col.name for col in colors]
            return 'Blue' in color_names

        def _is_blue_digimon_dp_2000(c) -> bool:
            if not _is_blue_digimon(c):
                return False
            dp = getattr(c, 'base_dp', None) or getattr(c, 'dp', None) or 0
            return dp <= 2000

        # --- Effect 0: [Main] Blue Digimon digivolves with cost -3, then place in battle area ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("LM-028 Blue Digimon digivolve cost -3, then place in battle area")
        effect0.set_effect_description(
            "[Main] 1 of your blue Digimon may digivolve into a blue Digimon "
            "card in the hand with the digivolution cost reduced by 3. Then, place "
            "this card in the battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def own_perm_filter(p):
                if not p.is_digimon:
                    return False
                top = p.top_card
                if not top:
                    return False
                colors = [c.name for c in (getattr(top, 'card_colors', None) or [])]
                return 'Blue' in colors

            def hand_card_filter(c):
                return _is_blue_digimon(c)

            def on_select_perm(selected_perm):
                game.effect_digivolve_from_hand(
                    player, selected_perm,
                    filter_fn=hand_card_filter,
                    cost_reduction=3,
                    is_optional=True
                )

            game.effect_select_own_permanent(
                player, on_select_perm,
                filter_fn=own_perm_filter,
                is_optional=True
            )

            # "Then, place this card in the battle area" — handled by the engine:
            # _option_stays_on_field returns True because _is_delay is set,
            # so the engine keeps this option card on the field after resolution.

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Delay marker ---
        effect1 = ICardEffect()
        effect1.set_effect_name("LM-028 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [Start of Your Turn] Delay effect ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartTurn)
        effect2.set_effect_name("LM-028 Delay: Return blue Digimon to deck top, then play")
        effect2.set_effect_description(
            "[Start of Your Turn] If your opponent has a Digimon, <Delay>. "
            "Return 1 blue Digimon card from your trash to the top of the deck. "
            "Then, if you don't have a Digimon, you may play 1 blue Digimon card "
            "with 2000 DP or less from your trash without paying the cost."
        )
        effect2._is_delay_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Note: _execute_delay trashes the card BEFORE calling this condition,
            # so permanent_of_this_card() would return None. Don't check it here.
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Opponent must have a Digimon
            enemy = card.owner.enemy if card and card.owner else None
            if not enemy:
                return False
            return any(p.is_digimon for p in enemy.battle_area)
        effect2.set_can_use_condition(condition2)

        # Trash selection action-ID offset (mirrors game.py SEL_TRASH_START)
        _SEL_TRASH_START = 130

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Note: the engine's _execute_delay already trashes this card
            # from the battle area before calling this callback.

            def _after_return():
                # Then, if you don't have a Digimon, play 1 blue Digimon DP <= 2000 from trash
                has_digimon = any(
                    p.is_digimon for p in (player.battle_area or [])
                )
                if not has_digimon:
                    game.effect_play_from_zone(
                        player, 'trash', _is_blue_digimon_dp_2000,
                        free=True, is_optional=True,
                        prompt="Select 1 blue Digimon with 2000 DP or less from trash to play."
                    )

            # Return 1 blue Digimon card from trash to top of deck (player selection)
            valid_trash = []
            for i, c in enumerate(player.trash_cards):
                if _is_blue_digimon(c):
                    valid_trash.append(_SEL_TRASH_START + i)

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
                    prompt="Select a blue Digimon from your trash to place at the top of your deck.",
                )
            else:
                _after_return()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] Play blue Digimon DP <= 2000 from trash, add this to hand ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("LM-028 Security: Play blue Digimon from trash")
        effect3.set_effect_description(
            "[Security] You may play 1 blue Digimon card with 2000 DP or less "
            "from your trash without paying the cost. Then, add this card to the hand."
        )
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            game.effect_play_from_zone(
                player, 'trash', _is_blue_digimon_dp_2000,
                free=True, is_optional=True,
                prompt="Select 1 blue Digimon with 2000 DP or less from trash to play."
            )

            # Then add this card to hand
            if card:
                player.hand_cards.append(card)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
