from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_087(CardScript):
    """BT5-087 Omnimon Zwart | Lv.7 Mega (Black, Vaccine, Holy Warrior)

    [When Digivolving] Trash the top 3 cards of your deck. Then, you may
        play up to 2 play cost 8 or less black or purple Digimon cards from
        your trash without paying the costs.
    [When Attacking] By returning 1 level 6 Digimon card in this Digimon's
        digivolution cards to the hand, delete 1 of your opponent's
        unsuspended Digimon with a play cost of 12 or less.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        from ....data.enums import CardColor
        effects = []

        # --- Effect 1: [When Digivolving] Trash 3 from deck, play up to 2 from trash ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT5-087 When Digivolving: Trash 3, play 2 from trash")
        effect1.set_effect_description(
            "[When Digivolving] Trash the top 3 cards of your deck. Then, you "
            "may play up to 2 play cost 8 or less black or purple Digimon cards "
            "from your trash without paying the costs."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Trash the top 3 cards of your deck
            for _ in range(min(3, len(player.library_cards))):
                trashed_card = player.library_cards.pop(0)
                player.trash_cards.append(trashed_card)

            # Then, play up to 2 black/purple Digimon with play cost 8 or less from trash
            def bp_digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                cost = getattr(c, 'play_cost', None)
                if cost is None or cost > 8:
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return CardColor.Black in colors or CardColor.Purple in colors

            # Play first
            game.effect_play_from_zone(
                player, 'trash', bp_digi_filter, free=True, is_optional=True)
            # Play second
            game.effect_play_from_zone(
                player, 'trash', bp_digi_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [When Attacking] Return Lv.6 digi-card to hand, delete opponent unsuspended ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("BT5-087 When Attacking: Return Lv.6, delete opponent")
        effect2.set_effect_description(
            "[When Attacking] By returning 1 level 6 Digimon card in this "
            "Digimon's digivolution cards to the hand, delete 1 of your "
            "opponent's unsuspended Digimon with a play cost of 12 or less."
        )
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            # Check this is the attacking permanent
            ctx_perm = None
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            enemy = player.enemy
            if not enemy:
                return

            # Cost: return 1 level 6 Digimon card from digivolution cards to hand
            lv6_sources = [
                cs for cs in perm.card_sources
                if cs is not perm.top_card
                and getattr(cs, 'is_digimon', False)
                and getattr(cs, 'level', None) == 6
            ]
            if not lv6_sources:
                return

            # Return the first qualifying lv6 card to hand
            chosen = lv6_sources[0]
            perm.card_sources.remove(chosen)
            player.hand_cards.append(chosen)

            # Delete 1 of opponent's unsuspended Digimon with play cost 12 or less
            def opp_filter(p):
                if not p.is_digimon:
                    return False
                if p.is_suspended:
                    return False
                tc = p.top_card
                if tc:
                    cost = getattr(tc, 'play_cost', None)
                    if cost is not None and cost > 12:
                        return False
                return True

            opp_targets = [p for p in enemy.battle_area if opp_filter(p)]
            if opp_targets:
                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)
                game.effect_select_opponent_permanent(
                    player, on_delete,
                    filter_fn=opp_filter,
                    is_optional=False,
                    prompt="Delete 1 of your opponent's unsuspended Digimon "
                           "with play cost 12 or less.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
