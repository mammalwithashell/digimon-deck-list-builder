from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_074(CardScript):
    """BT24-074 SkullSeadramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Lv.4 with [TS] or [Aqua] trait for cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-074 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ---------------------------------------------------------------
        # Shared On Play / When Digivolving:
        # Trash any 3 digivolution cards from 1 of your opponent's Digimon.
        # Then, if played by effects, delete 1 opponent's Digimon with no
        # digivolution cards.
        # ---------------------------------------------------------------

        def _trash_sources_process(ctx: Dict[str, Any]):
            """Trash up to 3 digi-sources from 1 opponent's Digimon, then conditionally delete sourceless."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Select 1 opponent's Digimon that has digivolution cards
            def opp_has_sources(p):
                if not p.is_digimon:
                    return False
                return not p.has_no_digivolution_cards

            def on_target_selected(target_perm):
                # Trash up to 3 digivolution cards from this Digimon
                count = min(3, len(target_perm.card_sources) - 1)  # don't trash the top card
                if count > 0:
                    trashed = target_perm.trash_digivolution_cards(count)
                    if enemy:
                        enemy.trash_cards.extend(trashed)

            game.effect_select_opponent_permanent(
                player, on_target_selected, filter_fn=opp_has_sources,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to trash digivolution cards from.")

            # Then, if this was played by an effect, delete 1 opponent's Digimon with no sources
            # NOTE: "if played by effects" condition requires context tracking;
            # for now we check if it was an On Play trigger (the C# checks IsByEffect + CanTriggerOnPlay)
            played_by_effect = ctx.get('played_by_effect', False)
            if played_by_effect:
                def no_sources_filter(p):
                    return p.is_digimon and p.has_no_digivolution_cards

                def on_delete_sourceless(target_perm):
                    if enemy:
                        enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete_sourceless, filter_fn=no_sources_filter,
                    is_optional=False,
                    prompt="Select 1 of your opponent's Digimon with no digivolution cards to delete.")

        # On Play
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT24-074 Trash 3 sources, if by effect, delete 1 sourceless")
        effect1.set_effect_description("[On Play] Trash any 3 digivolution cards from 1 of your opponent's Digimon. Then, if played by effects, delete 1 of your opponent's Digimon with no digivolution cards.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_trash_sources_process)
        effects.append(effect1)

        # When Digivolving
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-074 Trash 3 sources, if by effect, delete 1 sourceless")
        effect2.set_effect_description("[When Digivolving] Trash any 3 digivolution cards from 1 of your opponent's Digimon. Then, if played by effects, delete 1 of your opponent's Digimon with no digivolution cards.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_trash_sources_process)
        effects.append(effect2)

        # ---------------------------------------------------------------
        # On Deletion: Play 1 Lv.4- Digimon with [Seadramon] in name or
        # [TS] trait from TRASH (not hand) without paying cost.
        # ---------------------------------------------------------------
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("BT24-074 You may play 1 level 4 or lower Digimon")
        effect3.set_effect_description("[On Deletion] You may play 1 level 4 or lower Digimon card with [Seadramon] in its name or the [TS] trait from your trash without paying the cost.")
        effect3.is_optional = True
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Play 1 Lv.4- Seadramon/TS Digimon from trash free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                names = getattr(c, 'card_names', []) or []
                traits = getattr(c, 'card_traits', []) or []
                has_seadramon = any('Seadramon' in n for n in names)
                has_ts = any('TS' in t for t in traits)
                return has_seadramon or has_ts

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # ---------------------------------------------------------------
        # When Attacking ESS: [Once Per Turn] By placing 1 of your other
        # Digimon as this Digimon's bottom digivolution card, unsuspend
        # this Digimon.
        # ---------------------------------------------------------------
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("BT24-074 Place 1 of your other Digimon as this Digimon's bottom digivolution card to unsuspend this Digimon.")
        effect4.set_effect_description("[When Attacking] [Once Per Turn] By placing 1 of your other Digimon as this Digimon's bottom digivolution card, it unsuspends.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Attacking_BT24_074")
        effect4.is_on_attack = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Place 1 other own Digimon as bottom digi-source, then unsuspend this."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            this_perm = card.permanent_of_this_card() if card else perm

            def other_own_digi_filter(p):
                if not p.is_digimon:
                    return False
                if p is this_perm:
                    return False
                return True

            def on_selected(target_perm):
                # Remove the selected Digimon from battle area and place as bottom digi source
                if target_perm in player.battle_area:
                    player.battle_area.remove(target_perm)
                    # Place all cards from the consumed permanent as bottom digi sources
                    for cs in target_perm.card_sources:
                        this_perm.card_sources.insert(0, cs)
                    # Unsuspend this Digimon
                    if this_perm:
                        this_perm.unsuspend()

            game.effect_select_own_permanent(
                player, on_selected, filter_fn=other_own_digi_filter,
                is_optional=True,
                prompt="Select 1 of your other Digimon to place as this Digimon's bottom digivolution card.")

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
