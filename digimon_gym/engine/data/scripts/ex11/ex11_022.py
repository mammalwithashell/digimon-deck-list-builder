from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_022(CardScript):
    """EX11-022 Karakurumon | Lv.5

    Alt digivolve: Lv.4 with [Puppet] trait (Yellow/Purple) for cost 3.
    <Scapegoat>

    [On Play][When Digivolving] You may play 1 [Puppet] trait Digimon card with
        4000 DP or less from your hand or trash without paying the cost. At the
        end of the turn, delete the Digimon played by this effect.

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
        effect0.set_effect_name("EX11-022 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "Puppet"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Scapegoat ---
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-022 Scapegoat")
        effect1.set_effect_description("Scapegoat")
        effect1._is_scapegoat = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared: Play Puppet DP<=4000 from hand/trash, delete at end of turn ---
        def _play_puppet_and_schedule_delete(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                dp = getattr(c, 'base_dp', None) or getattr(c, 'dp', None)
                if dp is None or dp > 4000:
                    return False
                return _is_puppet_trait(c)

            # Track field before play
            before_perms = set(player.battle_area)
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True,
                prompt="You may play 1 [Puppet] Digimon with 4000 DP or less.")
            # Find the newly played permanent and schedule end-of-turn deletion
            after_perms = set(player.battle_area)
            new_perms = after_perms - before_perms
            for new_perm in new_perms:
                # Register an end-of-turn deletion effect
                eot_effect = ICardEffect()
                eot_effect.set_timing(EffectTiming.OnEndTurn)
                eot_effect.set_effect_name("EX11-022 Delete played Digimon at turn end")
                eot_effect.set_effect_description("Delete the Digimon played by this effect at end of turn.")
                _perm_ref = new_perm

                def eot_condition(context: Dict[str, Any], p=_perm_ref) -> bool:
                    return p in (card.owner.battle_area if card and card.owner else [])
                eot_effect.set_can_use_condition(eot_condition)

                def eot_process(ctx: Dict[str, Any], p=_perm_ref):
                    owner = card.owner if card else None
                    if owner and p in owner.battle_area:
                        owner.delete_permanent(p)
                eot_effect.set_on_process_callback(eot_process)

                if new_perm.top_card:
                    new_perm.top_card._card_effects.append(eot_effect)

        # --- Effect 2: [On Play] Play Puppet ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-022 Play Puppet DP<=4000, delete at turn end")
        effect2.set_effect_description("[On Play] Play 1 [Puppet] Digimon with 4000 DP or less from hand or trash. Delete it at turn end.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_play_puppet_and_schedule_delete)
        effects.append(effect2)

        # --- Effect 3: [When Digivolving] Play Puppet ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX11-022 Play Puppet DP<=4000, delete at turn end")
        effect3.set_effect_description("[When Digivolving] Play 1 [Puppet] Digimon with 4000 DP or less from hand or trash. Delete it at turn end.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_play_puppet_and_schedule_delete)
        effects.append(effect3)

        # --- Effect 4 (Inherited): WhenRemoveField - Delete Puppet/Token to prevent leaving ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.WhenRemoveField)
        effect4.set_effect_name("EX11-022 Delete Token/Puppet to prevent leaving")
        effect4.set_effect_description("[All Turns][Once Per Turn] When this Digimon would leave the battle area other than by your effects, by deleting 1 of your Tokens or other [Puppet] trait Digimon, prevent it from leaving.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Substitute_EX11_022")

        def condition4(context: Dict[str, Any]) -> bool:
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
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
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
                if my_perm and hasattr(my_perm, 'willBeRemoveField'):
                    my_perm.willBeRemoveField = False
                if my_perm and hasattr(my_perm, 'will_be_removed'):
                    my_perm.will_be_removed = False

            game.effect_select_own_permanent(
                player, on_delete_substitute, filter_fn=sub_filter,
                is_optional=True,
                prompt="Select 1 Token or [Puppet] Digimon to delete to prevent leaving.")

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
