from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_074(CardScript):
    """EX10-074 Beelzemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.3 w/ [Impmon] in name for cost 4, requires 20+ trash
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-074 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 4
        effect0._alt_digi_level = 3
        effect0._alt_digi_name = "Impmon"

        # Trash count condition: validator doesn't check can_use_condition for alt-digi
        # (card is in hand, not on field). Use _alt_digi_condition_fn instead.
        # base_perm.owner gives the player who owns the base Digimon.
        def _alt_digi_trash_check(base_perm) -> bool:
            owner = base_perm.owner if base_perm else None
            if owner is None and card:
                owner = card.owner
            return owner is not None and len(owner.trash_cards) >= 20
        effect0._alt_digi_condition_fn = _alt_digi_trash_check

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.owner:
                return len(card.owner.trash_cards) >= 20
            return False
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-074 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared logic for effects 2/3/4: Trash top 2 from deck, delete play cost 6+scaling ---
        def _first_effect_process(ctx: Dict[str, Any]):
            """[On Play]/[When Digivolving]/[When Attacking] Trash top 2, delete 1 opp Digimon play cost <= 6 + 3 per 10 trash cards."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Trash top 2 cards of deck
            player.mill(2)
            # Calculate play cost max: 6 + 3 * floor(trash_count / 10)
            trash_count = len(player.trash_cards)
            max_cost = 6 + 3 * (trash_count // 10)

            def target_filter(p):
                return (p.is_digimon and
                        hasattr(p.top_card, 'get_cost_itself') and
                        p.top_card.get_cost_itself <= max_cost)

            enemy = player.enemy
            if enemy:
                has_target = any(target_filter(p) for p in enemy.battle_area)
                if has_target:
                    def on_delete(target_perm):
                        enemy.delete_permanent(target_perm)
                    game.effect_select_opponent_permanent(
                        player, on_delete, filter_fn=target_filter, is_optional=False)

        # Effect 2: [On Play] trash top 2 + delete
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX10-074 Trash top 2, delete play cost 6 or lower")
        effect2.set_effect_description("[On Play] Trash the top 2 cards of your deck. Then, delete 1 of your opponent's play cost 6 or lower Digimon. For every 10 cards in your trash, add 3 to the play cost maximum.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_first_effect_process)
        effects.append(effect2)

        # Effect 3: [When Digivolving] trash top 2 + delete
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX10-074 Trash top 2, delete play cost 6 or lower")
        effect3.set_effect_description("[When Digivolving] Trash the top 2 cards of your deck. Then, delete 1 of your opponent's play cost 6 or lower Digimon. For every 10 cards in your trash, add 3 to the play cost maximum.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_first_effect_process)
        effects.append(effect3)

        # Effect 4: [When Attacking] trash top 2 + delete
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("EX10-074 Trash top 2, delete play cost 6 or lower")
        effect4.set_effect_description("[When Attacking] Trash the top 2 cards of your deck. Then, delete 1 of your opponent's play cost 6 or lower Digimon. For every 10 cards in your trash, add 3 to the play cost maximum.")
        effect4.is_on_attack = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_first_effect_process)
        effects.append(effect4)

        # --- Shared logic for effects 5/6: Return 2 from trash to deck top, De-Digivolve 2 ---
        def _second_effect_process(ctx: Dict[str, Any]):
            """[On Play]/[When Digivolving] If 10+ trash, by returning 2 non-Digi-Egg from trash to deck top, De-Digivolve 2 one opponent Digimon."""
            from ....game.constants import SEL_TRASH_START
            from ....data.enums import GamePhase as GP

            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if len(player.trash_cards) < 10:
                return
            # Count non-Digi-Egg cards in trash
            non_egg = [c for c in player.trash_cards if not getattr(c, 'is_digi_egg', False)]
            if len(non_egg) < 2:
                return
            # Check opponent has a Digimon target
            enemy = player.enemy
            if not enemy or not any(p.is_digimon for p in enemy.battle_area):
                return

            # "By returning 2 non-Digi-Egg cards from trash to deck top" is a cost
            returned_cards = []

            def trash_filter(c):
                return not getattr(c, 'is_digi_egg', False)

            def _build_valid_trash_indices():
                return [SEL_TRASH_START + i
                        for i, c in enumerate(player.trash_cards)
                        if trash_filter(c)]

            def on_first_selected(action_id: int):
                idx = action_id - SEL_TRASH_START
                if not (0 <= idx < len(player.trash_cards)):
                    return
                first_card = player.trash_cards[idx]
                player.trash_cards.remove(first_card)
                returned_cards.append(first_card)

                # Select second card from trash
                valid2 = _build_valid_trash_indices()
                if not valid2:
                    # Can't find a second card; revert first selection
                    player.trash_cards.append(first_card)
                    returned_cards.clear()
                    return

                def on_second_selected(action_id2: int):
                    idx2 = action_id2 - SEL_TRASH_START
                    if not (0 <= idx2 < len(player.trash_cards)):
                        return
                    second_card = player.trash_cards[idx2]
                    player.trash_cards.remove(second_card)
                    returned_cards.append(second_card)

                    # Place both on top of deck (reversed order per C#)
                    for rc in reversed(returned_cards):
                        player.library_cards.insert(0, rc)

                    # Now De-Digivolve 2 one opponent Digimon
                    def on_de_digivolve(target_perm):
                        removed = target_perm.de_digivolve(2)
                        if enemy:
                            enemy.trash_cards.extend(removed)
                    game.effect_select_opponent_permanent(
                        player, on_de_digivolve,
                        filter_fn=lambda p: p.is_digimon,
                        is_optional=False)

                game.request_selection(
                    GP.SelectTrash, player, on_second_selected, valid2,
                    is_optional=False,
                    prompt="Select a 2nd non-Digi-Egg card from trash to return to deck top.")

            valid1 = _build_valid_trash_indices()
            if not valid1:
                return
            game.request_selection(
                GP.SelectTrash, player, on_first_selected, valid1,
                is_optional=True,
                prompt="Select a non-Digi-Egg card from trash to return to deck top.")

        # Effect 5: [On Play] return 2 from trash, De-Digivolve 2
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("EX10-074 Return 2 from trash, De-Digivolve 2")
        effect5.set_effect_description("[On Play] If you have 10 or more cards in your trash, by returning 2 non-Digi-Egg cards from your trash to the top of the deck, <De-Digivolve 2> 1 of your opponent's Digimon.")
        effect5.is_on_play = True
        effect5.is_optional = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner:
                return len(card.owner.trash_cards) >= 10
            return False
        effect5.set_can_use_condition(condition5)
        effect5.set_on_process_callback(_second_effect_process)
        effects.append(effect5)

        # Effect 6: [When Digivolving] return 2 from trash, De-Digivolve 2
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect6.set_effect_name("EX10-074 Return 2 from trash, De-Digivolve 2")
        effect6.set_effect_description("[When Digivolving] If you have 10 or more cards in your trash, by returning 2 non-Digi-Egg cards from your trash to the top of the deck, <De-Digivolve 2> 1 of your opponent's Digimon.")
        effect6.is_when_digivolving = True
        effect6.is_optional = True

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner:
                return len(card.owner.trash_cards) >= 10
            return False
        effect6.set_can_use_condition(condition6)
        effect6.set_on_process_callback(_second_effect_process)
        effects.append(effect6)

        # Ace Overflow <-4> — engine handles via card data, descriptive effect
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("EX10-074 Ace Overflow <-4>")
        effect_ace.set_effect_description("Ace Overflow <-4>")
        effect_ace.is_inherited_effect = True

        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
