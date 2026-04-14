from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_066(CardScript):
    """BT24-066 Guilmon | Lv.3

    Alt digivolve: from [Gigimon] for cost 0
    [On Play] Reveal the top 3 cards of your deck. Among them, add 1
        [Evil], [Dark Dragon], [Evil Dragon] or [Dark Knight] trait card
        or purple Tamer card to the hand and trash 1 such card. Return
        the rest to the bottom of the deck. Then, trash 1 card in your hand.
    Inherited: [When Attacking] [Once Per Turn] Delete 1 of your opponent's
        level 3 Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from [Gigimon] for cost 0 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-066 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Gigimon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Gigimon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Reveal 3, add 1, trash 1 from revealed,
        #     return rest to bottom, then trash 1 from hand ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT24-066 Reveal 3, add 1, trash 1, return rest, trash 1 from hand")
        effect1.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Among them, add 1 "
            "[Evil], [Dark Dragon], [Evil Dragon] or [Dark Knight] trait card or "
            "purple Tamer card to the hand and trash 1 such card. Return the rest "
            "to the bottom of the deck. Then, trash 1 card in your hand.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Reveal 3 from deck. Among qualifying cards, add 1 to hand
            and trash 1 (two sequential selections from the revealed pool).
            Return rest to deck bottom. Then trash 1 from hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            from ....data.enums import CardColor, GamePhase

            def _qualifies(c):
                traits = getattr(c, 'card_traits', []) or []
                has_trait = any(
                    t == 'Evil' or t == 'Dark Dragon' or
                    t == 'Evil Dragon' or t == 'Dark Knight'
                    for t in traits)
                is_purple_tamer = (
                    getattr(c, 'is_tamer', False) and
                    CardColor.Purple in (getattr(c, 'card_colors', []) or []))
                return has_trait or is_purple_tamer

            def _trash_from_hand():
                """After reveal resolves, trash 1 card from hand."""
                if player.hand_cards:
                    def on_trashed(selected):
                        if selected in player.hand_cards:
                            player.hand_cards.remove(selected)
                            player.trash_cards.append(selected)
                    game.effect_select_hand_card(
                        player,
                        filter_fn=lambda c: True,
                        callback=on_trashed,
                        is_optional=False,
                        prompt="Trash 1 card from your hand.")

            # Reveal top 3, then do 2 selection passes, then trash from hand.
            # We manually implement the reveal flow to chain trash-from-hand
            # into the completion callback (effect_reveal_and_select_multi
            # doesn't support a completion callback).
            SEL_REVEALED_START = 30  # engine constant from game/constants.py

            revealed = player.library_cards[:3]
            if not revealed:
                _trash_from_hand()
                return

            for c in revealed:
                player.library_cards.remove(c)

            pool = list(revealed)

            passes = [
                (_qualifies, 'hand'),   # add 1 to hand
                (_qualifies, 'trash'),  # trash 1
            ]

            def run_pass(pass_idx):
                if pass_idx >= len(passes) or not pool:
                    # All passes done — return remaining to deck bottom
                    game.revealed_cards = []
                    for c in pool:
                        player.library_cards.append(c)
                    pool.clear()
                    # Chain: trash 1 from hand
                    _trash_from_hand()
                    return

                filter_fn, placement = passes[pass_idx]
                game.revealed_cards = list(pool)

                valid = []
                for i, card_in_pool in enumerate(pool):
                    if filter_fn(card_in_pool):
                        valid.append(SEL_REVEALED_START + i)

                if not valid:
                    run_pass(pass_idx + 1)
                    return

                def on_select(action_id):
                    idx = action_id - SEL_REVEALED_START
                    if 0 <= idx < len(pool):
                        selected = pool.pop(idx)
                        if placement == 'hand':
                            player.hand_cards.append(selected)
                        elif placement == 'trash':
                            player.trash_cards.append(selected)
                    game.revealed_cards = []
                    run_pass(pass_idx + 1)

                def on_decline():
                    game.revealed_cards = []
                    run_pass(pass_idx + 1)

                game.request_selection(
                    GamePhase.SelectReveal, player, on_select, valid,
                    is_optional=True,
                    prompt=f"Select a card from the revealed cards (pass {pass_idx + 1}).",
                    on_decline=on_decline)

            run_pass(0)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited [When Attacking] [Once Per Turn]
        #     Delete 1 opponent's level 3 Digimon ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("BT24-066 Delete opponent's level 3 Digimon")
        effect2.set_effect_description(
            "[When Attacking] [Once Per Turn] Delete 1 of your opponent's level 3 Digimon.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_066_Inherited")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete 1 of opponent's level 3 Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def target_filter(p):
                return (p.is_digimon and
                        p.level is not None and p.level == 3)

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            if any(target_filter(p) for p in enemy.battle_area):
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter,
                    is_optional=False,
                    prompt="Delete 1 of your opponent's level 3 Digimon.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
