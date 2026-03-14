from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_028(CardScript):
    """BT24-028 Divermon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.4 with [Aqua] or [TS] trait for cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-028 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        # C# checks HasAquaTraits OR HasTSTraits — use multi-trait alt digi
        effect0._alt_digi_trait = "Aqua|TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Shared helper: tuck filter for On Play / When Digivolving
        # C# CardCondition: IsDigimon && Level <= 5 && Blue && HasTSTraits
        def _tuck_card_ok(c) -> bool:
            if not getattr(c, 'is_digimon', False):
                return False
            level = getattr(c, 'level', None)
            if level is None or level > 5:
                return False
            colors = [col.name for col in (getattr(c, 'card_colors', None) or [])]
            if 'Blue' not in colors:
                return False
            traits = getattr(c, 'card_traits', []) or []
            return any('TS' in t for t in traits)

        def _shared_process(ctx: Dict[str, Any]):
            """By tucking a Lv.5- blue TS Digimon from hand, gain Blocker + cannot be deleted in battle until opp turn ends."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def on_tuck(selected):
                if selected is None:
                    return
                # Place selected card as bottom digivolution card
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                perm.add_card_source_bottom(selected)
                # Grant Blocker and cannot-be-deleted-by-battle until opponent's turn ends
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm, ModifierType.GRANT_BLOCKER,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
                game.register_modifier(
                    perm, ModifierType.CANNOT_BE_DESTROYED_BY_BATTLE,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')

            game.effect_select_hand_card(
                player, _tuck_card_ok, on_tuck, is_optional=True,
                prompt="Select 1 level 5 or lower blue [TS] Digimon to place as bottom digivolution card.")

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 level 5 or lower blue [TS] trait Digimon card from your hand as this Digimon's bottom digivolution card, until your opponent's turn ends, this Digimon can't be deleted in battle and gains <Blocker>
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT24-028 By placing 1 level 5 or lower [TS digimon] as bottom source card, gain battle immunity & <Blocker>")
        effect1.set_effect_description("[On Play] By placing 1 level 5 or lower blue [TS] trait Digimon card from your hand as this Digimon's bottom digivolution card, until your opponent's turn ends, this Digimon can't be deleted in battle and gains <Blocker>")
        effect1.is_on_play = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            return any(_tuck_card_ok(c) for c in player.hand_cards)

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_shared_process)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing 1 level 5 or lower blue [TS] trait Digimon card from your hand as this Digimon's bottom digivolution card, until your opponent's turn ends, this Digimon can't be deleted in battle and gains <Blocker>
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-028 By placing 1 level 5 or lower [TS digimon] as bottom source card, gain battle immunity & <Blocker>")
        effect2.set_effect_description("[When Digivolving] By placing 1 level 5 or lower blue [TS] trait Digimon card from your hand as this Digimon's bottom digivolution card, until your opponent's turn ends, this Digimon can't be deleted in battle and gains <Blocker>")
        effect2.is_when_digivolving = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            return any(_tuck_card_ok(c) for c in player.hand_cards)

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_shared_process)
        effects.append(effect2)

        # Timing: EffectTiming.OnUnTappedAnyone
        # [Your Turn] When this Digimon unsuspends, it may digivolve into [Neptunemon] in the hand without paying the cost.
        # C# condition: CanTriggerWhenSelfPermanentUnsuspends (self unsuspend trigger) + HasMatchConditionOwnersHand for Neptunemon + IsOwnerTurn
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUnTappedAnyone)
        effect3.set_effect_name("BT24-028 You may digivolve into [Neptunemon]")
        effect3.set_effect_description("[Your Turn] When this Digimon unsuspends, it may digivolve into [Neptunemon] in the hand without paying the cost.")
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be owner's turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must be self-unsuspend trigger
            unsuspended_perm = context.get('permanent') or context.get('unsuspended_permanent')
            my_perm = card.permanent_of_this_card() if card else None
            if unsuspended_perm and my_perm and unsuspended_perm is not my_perm:
                return False
            # Must have Neptunemon in hand
            player = card.owner if card else None
            if not player:
                return False
            return any(
                any('Neptunemon' in _n for _n in getattr(c, 'card_names', []))
                for c in player.hand_cards
            )

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Digivolve into Neptunemon from hand for free."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return any('Neptunemon' in _n for _n in getattr(c, 'card_names', []))
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, cost_override=0, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnUseAttack (inherited When Attacking)
        # [When Attacking] [Once Per Turn] You may play 1 level 4 or lower blue Digimon card with the [TS] trait from this Digimon's digivolution cards without paying the cost.
        # NOTE: C# CardCondition does NOT check HasTSTraits — only IsDigimon + Blue + Level<=4
        # However card text specifies [TS] trait. Keep TS check to match card text.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("BT24-028 Play 1 level 4 or lower blue [TS] digimon in digivolution sources")
        effect4.set_effect_description("[When Attacking] [Once Per Turn] You may play 1 level 4 or lower blue Digimon card with the [TS] trait from this Digimon's digivolution cards without paying the cost.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_028_ESS")
        effect4.is_on_attack = True

        def _digi_source_filter(cs, check_ts=True) -> bool:
            if not getattr(cs, 'is_digimon', False):
                return False
            level = getattr(cs, 'level', None)
            if level is None or level > 4:
                return False
            colors = [c.name for c in (getattr(cs, 'card_colors', None) or [])]
            if 'Blue' not in colors:
                return False
            if check_ts:
                traits = getattr(cs, 'card_traits', []) or []
                if not any('TS' in t for t in traits):
                    return False
            return True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return False
            top = perm.top_card
            for cs in perm.card_sources:
                if cs is top:
                    continue
                if _digi_source_filter(cs):
                    return True
            return False

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Play Lv.4 or lower blue [TS] Digimon from digi sources (player selects)."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            from ....data.enums import GamePhase
            from ....game.constants import SOURCES_PER_FIELD
            # Find field index of this permanent
            field_idx = None
            for fi, fp in enumerate(player.battle_area):
                if fp is perm:
                    field_idx = fi
                    break
            if field_idx is None:
                return
            base = 2000 + field_idx * SOURCES_PER_FIELD
            top = perm.top_card
            valid = []
            for i, cs in enumerate(perm.card_sources):
                if cs is top:
                    continue
                if _digi_source_filter(cs) and (base + i) < 2168:
                    valid.append(base + i)
            if not valid:
                return

            def on_source_selected(action_id):
                idx = action_id - base
                if not (0 <= idx < len(perm.card_sources)):
                    return
                selected = perm.card_sources[idx]
                perm.card_sources.remove(selected)
                played = player.play_card_from_source(selected, pay_cost=False)
                game.execute_effects(
                    EffectTiming.OnEnterFieldAnyone,
                    {"played_card": selected, "played_permanent": played,
                     "event_player": player},
                )

            game.request_selection(
                GamePhase.SelectSource, player, on_source_selected,
                valid, is_optional=True,
                prompt="Select 1 level 4 or lower blue [TS] Digimon from digivolution cards to play.")

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
