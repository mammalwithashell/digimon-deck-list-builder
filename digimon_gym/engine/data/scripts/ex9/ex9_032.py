from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_032(CardScript):
    """EX9-032 Karakurumon | Lv.5 Yellow | Puppet

    Alt digivolve: from Lv.4 [Puppet] for cost 3.

    [On Play] By deleting 1 of your Tokens or other [Puppet] trait Digimon,
        this Digimon may digivolve into a Digimon card with the [Puppet] trait
        in the hand without paying the cost.

    [When Digivolving] By deleting 1 of your Tokens or other [Puppet] trait
        Digimon, this Digimon may digivolve into a Digimon card with the
        [Puppet] trait in the hand without paying the cost.

    --- Inherited ---
    [All Turns][Once Per Turn] When this Digimon would leave the battle area
        other than by your effects, by deleting 1 of your Tokens or other
        [Puppet] trait Digimon, prevent it from leaving.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_puppet_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Puppet' in t for t in traits)

        # --- Effect 0: Alt digivolve from Lv.4 [Puppet] for cost 3 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-032 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "Puppet"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Shared: Delete token/other Puppet Digimon, then digivolve into Puppet from hand free ---
        def _delete_and_digivolve(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return

            def delete_target_filter(p):
                if not p.is_digimon:
                    return False
                if p is my_perm:
                    return False
                if getattr(p, 'is_token', False):
                    return True
                if p.top_card and _is_puppet_trait(p.top_card):
                    return True
                return False

            def hand_card_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return _is_puppet_trait(c)

            def on_delete(target_perm):
                player.delete_permanent(target_perm)

                # Then digivolve this Digimon into a [Puppet] from hand for free
                my_perm_now = card.permanent_of_this_card() if card else None
                if not my_perm_now:
                    return
                game.effect_digivolve_from_hand(
                    player, my_perm_now,
                    filter_fn=hand_card_filter,
                    cost_override=0,
                    is_optional=True
                )

            has_target = any(delete_target_filter(p) for p in player.battle_area)
            if not has_target:
                return

            game.effect_select_own_permanent(
                player, on_delete, filter_fn=delete_target_filter,
                is_optional=True,
                prompt="Select 1 of your Tokens or other [Puppet] Digimon to delete."
            )

        # --- Effect 1: [On Play] Delete token/Puppet, digivolve into Puppet from hand ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-032 On Play: Delete + digivolve into Puppet")
        effect1.set_effect_description(
            "[On Play] By deleting 1 of your Tokens or other [Puppet] trait Digimon, "
            "this Digimon may digivolve into a Digimon card with the [Puppet] trait "
            "in the hand without paying the cost."
        )
        effect1.is_on_play = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_delete_and_digivolve)
        effects.append(effect1)

        # --- Effect 2: [When Digivolving] Delete token/Puppet, digivolve into Puppet from hand ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX9-032 When Digivolving: Delete + digivolve into Puppet")
        effect2.set_effect_description(
            "[When Digivolving] By deleting 1 of your Tokens or other [Puppet] trait "
            "Digimon, this Digimon may digivolve into a Digimon card with the [Puppet] "
            "trait in the hand without paying the cost."
        )
        effect2.is_when_digivolving = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_delete_and_digivolve)
        effects.append(effect2)

        # --- Effect 3 (Inherited): [All Turns][Once Per Turn] Prevent leaving by deleting token/Puppet ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenRemoveField)
        effect3.set_effect_name("EX9-032 Prevent leaving by deleting token/Puppet")
        effect3.set_effect_description(
            "[All Turns][Once Per Turn] When this Digimon would leave the battle area "
            "other than by your effects, by deleting 1 of your Tokens or other [Puppet] "
            "trait Digimon, prevent it from leaving."
        )
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Substitute_EX9_032")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            my_perm = card.permanent_of_this_card()

            def sub_filter(p):
                if not p.is_digimon:
                    return False
                if p is my_perm:
                    return False
                if getattr(p, 'is_token', False):
                    return True
                if p.top_card and _is_puppet_trait(p.top_card):
                    return True
                return False

            return any(sub_filter(p) for p in player.battle_area)
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return

            def sub_filter(p):
                if not p.is_digimon:
                    return False
                if p is my_perm:
                    return False
                if getattr(p, 'is_token', False):
                    return True
                if p.top_card and _is_puppet_trait(p.top_card):
                    return True
                return False

            def on_delete_substitute(target_perm):
                player.delete_permanent(target_perm)
                # Prevent this Digimon from leaving the battle area
                if my_perm and hasattr(my_perm, 'willBeRemoveField'):
                    my_perm.willBeRemoveField = False
                if my_perm and hasattr(my_perm, 'will_be_removed'):
                    my_perm.will_be_removed = False

            game.effect_select_own_permanent(
                player, on_delete_substitute, filter_fn=sub_filter,
                is_optional=True,
                prompt="Select 1 of your Tokens or other [Puppet] Digimon to delete to prevent leaving."
            )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
