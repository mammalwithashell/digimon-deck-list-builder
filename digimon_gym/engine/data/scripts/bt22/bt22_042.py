from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_042(CardScript):
    """BT22-042 Nyabootmon | Lv.7

    Alt digivolve: from [Chaperomon] for cost 6 (if you have [Arisa Kinosaki]).
    <Overclock ([Puppet] Trait)>

    [When Digivolving] You may play 1 level 4 or lower [Puppet] trait Digimon
        card from your hand without paying the cost. Then, to 1 of your
        opponent's Digimon, give -3000 DP until their turn ends for each of
        your Digimon.

    [All Turns][Once Per Turn] When any of your other Digimon are deleted, you
        may activate 1 of this Digimon's [When Digivolving] effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_puppet_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Puppet' in t for t in traits)

        # --- Effect 0: Alt digivolve from [Chaperomon] for cost 6 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-042 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 6
        effect0._alt_digi_name = "Chaperomon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.contains_card_name('Chaperomon')):
                return False
            # Requires having Arisa Kinosaki
            player = card.owner if card else None
            if player:
                has_arisa = any(
                    p.is_tamer and p.top_card and
                    any('Arisa Kinosaki' in n for n in getattr(p.top_card, 'card_names', []))
                    for p in player.battle_area
                )
                if not has_arisa:
                    return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Overclock ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-042 Overclock")
        effect1.set_effect_description("Overclock")
        effect1._is_overclock = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared: When Digivolving effect (play Puppet + DP reduction) ---
        def _when_digivolving_effect(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Play 1 level 4 or lower Puppet Digimon from hand
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                level = getattr(c, 'level', None)
                if level is None or level > 4:
                    return False
                return _is_puppet_trait(c)
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True,
                prompt="You may play 1 level 4 or lower [Puppet] Digimon from hand.")
            # Then give -3000 DP per own Digimon to 1 opponent's Digimon
            enemy = player.enemy if player else None
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return
            own_digimon_count = sum(1 for p in player.battle_area if p.is_digimon)
            dp_change = -3000 * own_digimon_count
            if dp_change == 0:
                return

            def target_filter(p):
                return p.is_digimon

            def on_select(target_perm):
                target_perm.change_dp(dp_change)

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False,
                prompt=f"Select 1 opponent's Digimon to give {dp_change} DP.")

        # --- Effect 2: [When Digivolving] Play Puppet + -3000 DP ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-042 Play Puppet + DP reduction")
        effect2.set_effect_description("[When Digivolving] Play Puppet Lv4- from hand, then -3000 DP per Digimon to 1 opponent's Digimon.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_when_digivolving_effect)
        effects.append(effect2)

        # --- Effect 3: [All Turns][Once Per Turn] On other Digimon deletion, re-activate WD ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("BT22-042 Activate a [When Digivolving] effect")
        effect3.set_effect_description("[All Turns][Once Per Turn] When any of your other Digimon are deleted, you may activate 1 of this Digimon's [When Digivolving] effects.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT22_042_UseWD")
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The deleted permanent must be one of our other Digimon
            deleted_perm = context.get('permanent')
            if deleted_perm:
                my_perm = card.permanent_of_this_card()
                if deleted_perm is my_perm:
                    return False
                if not deleted_perm.is_digimon:
                    return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            # Re-activate the When Digivolving effect
            _when_digivolving_effect(ctx)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
