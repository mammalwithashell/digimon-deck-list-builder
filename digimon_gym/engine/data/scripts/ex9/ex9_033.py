from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_033(CardScript):
    """EX9-033 Kaguyamon | Lv.6 Yellow | Puppet

    Alt digivolve: from Lv.5 [Puppet] for cost 3.

    [All Turns] All of your Tokens and [Puppet] trait Digimon gain <Blocker>
        and <Alliance>.

    [All Turns][Once Per Turn] When other Digimon are deleted, delete 1 of
        your opponent's lowest level Digimon.

    [End of Your Turn][Once Per Turn] You may play 1 level 4 or lower [Puppet]
        trait Digimon card from your trash without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_puppet_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Puppet' in t for t in traits)

        def _is_own_puppet_or_token(perm) -> bool:
            """Check if a permanent is one of our Tokens or [Puppet] Digimon."""
            if not perm.is_digimon:
                return False
            player = card.owner if card else None
            if not player:
                return False
            if perm not in player.battle_area:
                return False
            if getattr(perm, 'is_token', False):
                return True
            if perm.top_card and _is_puppet_trait(perm.top_card):
                return True
            return False

        # --- Effect 0: Alt digivolve from Lv.5 [Puppet] for cost 3 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-033 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "Puppet"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [All Turns] Grant Blocker to own Tokens and [Puppet] Digimon ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.NoTiming)
        effect1.set_effect_name("EX9-033 Grant Blocker to Puppets/Tokens")
        effect1.set_effect_description(
            "[All Turns] All of your Tokens and [Puppet] trait Digimon gain <Blocker>."
        )
        effect1._is_blocker = True
        effect1.is_declarative = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            return _is_own_puppet_or_token(perm)
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [All Turns] Grant Alliance to own Tokens and [Puppet] Digimon ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.NoTiming)
        effect2.set_effect_name("EX9-033 Grant Alliance to Puppets/Tokens")
        effect2.set_effect_description(
            "[All Turns] All of your Tokens and [Puppet] trait Digimon gain <Alliance>."
        )
        effect2._is_alliance = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            return _is_own_puppet_or_token(perm)
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Effect 3: [All Turns][Once Per Turn] When other Digimon deleted,
        #     delete 1 opponent's lowest level Digimon ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("EX9-033 Delete opponent's lowest level Digimon")
        effect3.set_effect_description(
            "[All Turns][Once Per Turn] When other Digimon are deleted, delete 1 "
            "of your opponent's lowest level Digimon."
        )
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX9_033_Deletion")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The deleted Digimon must be a different Digimon (not this one)
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

            # Find lowest level among opponent's Digimon
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

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: [End of Your Turn][Once Per Turn] Play Lv.4 or lower [Puppet] from trash free ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEndTurn)
        effect4.set_effect_name("EX9-033 Play Puppet from trash")
        effect4.set_effect_description(
            "[End of Your Turn][Once Per Turn] You may play 1 level 4 or lower "
            "[Puppet] trait Digimon card from your trash without paying the cost."
        )
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("EX9_033_PlayPuppet")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def puppet_lv4_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                level = getattr(c, 'level', None) or 0
                if level > 4:
                    return False
                return _is_puppet_trait(c)

            game.effect_play_from_zone(
                player, 'trash', puppet_lv4_filter, free=True, is_optional=True,
                prompt="Select 1 level 4 or lower [Puppet] Digimon from trash to play."
            )

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
