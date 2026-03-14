from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_023(CardScript):
    """EX11-023 Kaguyamon | Lv.6

    Alt digivolve: Lv.5 with [Puppet] trait for cost 3.
    <Alliance>
    <Scapegoat>

    [When Digivolving][Once Per Turn] Delete 1 of your opponent's Digimon
        with the lowest level.

    [End of Opponent's Turn][Once Per Turn] Delete 1 of your opponent's Digimon
        with the lowest level.

    [All Turns][Once Per Turn] When other Digimon are deleted, you may play 1
        level 4 or lower [Puppet] trait Digimon card from your trash without
        paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_puppet_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Puppet' in t for t in traits)

        # --- Effect 0: Alt digivolve ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-023 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "Puppet"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Alliance ---
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-023 Alliance")
        effect1.set_effect_description("Alliance")
        effect1.is_on_attack = True
        effect1._is_alliance = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: Scapegoat ---
        effect2 = ICardEffect()
        effect2.set_effect_name("EX11-023 Scapegoat")
        effect2.set_effect_description("Scapegoat")
        effect2._is_scapegoat = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Shared: Delete opponent's lowest level Digimon ---
        def _delete_lowest_level(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return

            def get_level(p):
                if p.top_card:
                    return getattr(p.top_card, 'level', 99) or 99
                return 99

            min_level = min(get_level(p) for p in opp_digimon)
            lowest_level = [p for p in opp_digimon if get_level(p) == min_level]

            def lowest_filter(p):
                return p in lowest_level

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=lowest_filter,
                is_optional=False,
                prompt="Select 1 of your opponent's lowest level Digimon to delete."
            )

        # --- Effect 3: [When Digivolving][Once Per Turn] Delete lowest level ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX11-023 Delete lowest level")
        effect3.set_effect_description("[When Digivolving][Once Per Turn] Delete 1 opponent's lowest level Digimon.")
        effect3.is_when_digivolving = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX11_023_WD")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_delete_lowest_level)
        effects.append(effect3)

        # --- Effect 4: [End of Opponent's Turn][Once Per Turn] Delete lowest level ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEndTurn)
        effect4.set_effect_name("EX11-023 Delete lowest level")
        effect4.set_effect_description("[End of Opponent's Turn][Once Per Turn] Delete 1 opponent's lowest level Digimon.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("EX11_023_WD_EoOT")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only on opponent's turn
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_delete_lowest_level)
        effects.append(effect4)

        # --- Effect 5: [All Turns][Once Per Turn] On other deletion, play Puppet Lv4- from trash ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnDestroyedAnyone)
        effect5.set_effect_name("EX11-023 Play level 4 or lower [Puppet] from trash")
        effect5.set_effect_description("[All Turns][Once Per Turn] When other Digimon are deleted, you may play 1 level 4 or lower [Puppet] trait Digimon card from your trash without paying the cost.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("EX11_023_AT")
        effect5.is_on_deletion = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The deleted Digimon must be different from this one
            deleted_perm = context.get('permanent')
            if deleted_perm:
                my_perm = card.permanent_of_this_card()
                if deleted_perm is my_perm:
                    return False
                if not deleted_perm.is_digimon:
                    return False
            return True
        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                level = getattr(c, 'level', None)
                if level is None or level > 4:
                    return False
                return _is_puppet_trait(c)

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True,
                prompt="Play 1 level 4 or lower [Puppet] Digimon from trash.")

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
