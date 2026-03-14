from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_050(CardScript):
    """EX11-050 Loudmon | Lv.5

    Alt digivolve: Lv.4 with [Dark Dragon] or [Evil Dragon] trait for cost 3
    [On Play] [When Digivolving] Trash 2 cards in your hand. Then, delete
        1 of your opponent's Digimon with as much or less DP as 1 of your
        [Dark Dragon] or [Evil Dragon] trait Digimon.
    [All Turns] While you have 4 or fewer cards in your hand, all of your
        [Dark Dragon] or [Evil Dragon] trait Digimon gain <Scapegoat>.
    Inherited: [Your Turn] While you have 4 or fewer cards in your hand,
        all of your [Dark Dragon] or [Evil Dragon] trait Digimon gain
        <Security A. +1>.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Lv.4 [Dark Dragon] or [Evil Dragon] for cost 3 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-050 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "Dark Dragon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not permanent or not permanent.top_card:
                return False
            traits = getattr(permanent.top_card, 'card_traits', []) or []
            return (any('Dark Dragon' in tr for tr in traits) or
                    any('Evil Dragon' in tr for tr in traits))
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Shared On Play / When Digivolving logic ---
        def _shared_process(ctx: Dict[str, Any]):
            """Trash 2 cards from hand. Then select 1 of your [Dark Dragon]/
            [Evil Dragon] Digimon as reference, then delete 1 opponent Digimon
            with DP <= that Digimon's DP."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Trash up to 2 cards from hand (mandatory)
            _trash_count = [0]

            def _trash_one_then_continue():
                if _trash_count[0] >= 2 or not player.hand_cards:
                    _do_delete_step()
                    return

                def on_trashed(selected):
                    if selected in player.hand_cards:
                        player.hand_cards.remove(selected)
                        player.trash_cards.append(selected)
                    _trash_count[0] += 1
                    _trash_one_then_continue()

                game.effect_select_hand_card(
                    player, lambda c: True, on_trashed,
                    is_optional=False,
                    prompt=f"Trash card {_trash_count[0]+1}/2 from your hand.")

            def _do_delete_step():
                """Step 2: Select 1 of your [Dark Dragon]/[Evil Dragon] Digimon,
                then delete 1 opponent Digimon with DP <= that Digimon's DP."""
                enemy = player.enemy
                if not enemy:
                    return

                def _has_trait(p):
                    if not p.is_digimon or not p.top_card:
                        return False
                    traits = getattr(p.top_card, 'card_traits', []) or []
                    return any('Dark Dragon' in t or 'Evil Dragon' in t for t in traits)

                # Find own qualifying Digimon
                own_qualifying = [p for p in player.battle_area if _has_trait(p)]
                if not own_qualifying:
                    return

                # Check if any opponent Digimon could be deleted
                def _has_deletable_target():
                    for own_p in own_qualifying:
                        own_dp = own_p.dp or 0
                        for opp_p in enemy.battle_area:
                            if opp_p.is_digimon and opp_p.dp is not None and opp_p.dp <= own_dp:
                                return True
                    return False

                if not _has_deletable_target():
                    return

                def on_own_selected(own_perm):
                    """After selecting reference Digimon, delete 1 opp with DP <= own DP."""
                    if own_perm is None:
                        return
                    ref_dp = own_perm.dp or 0

                    def opp_filter(p):
                        return (p.is_digimon and
                                p.dp is not None and p.dp <= ref_dp)

                    def on_opp_delete(target_perm):
                        enemy.delete_permanent(target_perm)

                    if any(opp_filter(p) for p in enemy.battle_area):
                        game.effect_select_opponent_permanent(
                            player, on_opp_delete,
                            filter_fn=opp_filter,
                            is_optional=False,
                            prompt=f"Delete 1 opponent Digimon with {ref_dp} DP or less.")

                game.effect_select_own_permanent(
                    player, on_own_selected,
                    filter_fn=_has_trait,
                    is_optional=False,
                    prompt="Select 1 of your [Dark Dragon]/[Evil Dragon] Digimon as reference.")

            _trash_one_then_continue()

        # --- Effect 1: [On Play] ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-050 Trash 2, delete by DP comparison")
        effect1.set_effect_description(
            "[On Play] Trash 2 cards in your hand. Then, delete 1 of your "
            "opponent's Digimon with as much or less DP as 1 of your [Dark Dragon] "
            "or [Evil Dragon] trait Digimon.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_shared_process)
        effects.append(effect1)

        # --- Effect 2: [When Digivolving] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-050 Trash 2, delete by DP comparison")
        effect2.set_effect_description(
            "[When Digivolving] Trash 2 cards in your hand. Then, delete 1 of your "
            "opponent's Digimon with as much or less DP as 1 of your [Dark Dragon] "
            "or [Evil Dragon] trait Digimon.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_shared_process)
        effects.append(effect2)

        # --- Effect 3: [All Turns] While hand <= 4, [Dark Dragon]/[Evil Dragon]
        #     Digimon gain Scapegoat ---
        effect3 = ICardEffect()
        effect3.set_effect_name("EX11-050 Grant Scapegoat to Dark/Evil Dragon")
        effect3.set_effect_description(
            "[All Turns] While you have 4 or fewer cards in your hand, all of "
            "your [Dark Dragon] or [Evil Dragon] trait Digimon gain <Scapegoat>.")
        effect3._is_scapegoat = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card()
            if not permanent or not permanent.top_card:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            if len(owner.hand_cards) > 4:
                return False
            traits = getattr(permanent.top_card, 'card_traits', []) or []
            return (any('Dark Dragon' in tr for tr in traits) or
                    any('Evil Dragon' in tr for tr in traits))
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # --- Effect 4: Inherited [Your Turn] While hand <= 4, [Dark Dragon]/
        #     [Evil Dragon] Digimon gain SA+1 ---
        effect4 = ICardEffect()
        effect4.set_effect_name("EX11-050 Grant SA+1 to Dark/Evil Dragon")
        effect4.set_effect_description(
            "[Your Turn] While you have 4 or fewer cards in your hand, all of "
            "your [Dark Dragon] or [Evil Dragon] trait Digimon gain <Security A. +1>.")
        effect4.is_inherited_effect = True
        effect4.sa_modifier = 1

        def condition4(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner
            if len(owner.hand_cards) > 4:
                return False
            permanent = card.permanent_of_this_card()
            if not permanent or not permanent.top_card:
                return False
            traits = getattr(permanent.top_card, 'card_traits', []) or []
            return (any('Dark Dragon' in tr for tr in traits) or
                    any('Evil Dragon' in tr for tr in traits))
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
