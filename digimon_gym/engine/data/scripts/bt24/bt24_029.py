from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_029(CardScript):
    """BT24-029 Whamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-029 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 with [TS] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 play cost 5 or lower card with the [Sea Beast] or [TS] trait or [Aqua] or [Sea Animal] in any of its traits from your hand as this Digimon's bottom digivolution card, 1 of your opponent's Digimon or Tamers can't suspend until their turn ends.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT24-029 By placing 1 5 or lower play cost [TS]/[Sea Beast]/[Aqua]/[Sea Animal] card as bottom source, 1 digimon/tamer cant suspend")
        effect1.set_effect_description("[On Play] By placing 1 play cost 5 or lower card with the [Sea Beast] or [TS] trait or [Aqua] or [Sea Animal] in any of its traits from your hand as this Digimon's bottom digivolution card, 1 of your opponent's Digimon or Tamers can't suspend until their turn ends.")
        effect1.is_on_play = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Check eligible hand cards to tuck
            def _tuck_ok(c):
                cost = getattr(c, 'play_cost', None)
                if cost is None or cost > 5:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return (any(t in ('Sea Beast', 'Aqua', 'Sea Animal') for t in traits)
                        or any('TS' in t for t in traits))
            return any(_tuck_ok(c) for c in player.hand_cards)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """By tucking a card from hand, freeze 1 opponent perm."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def tuck_filter(c):
                cost = getattr(c, 'play_cost', None)
                if cost is None or cost > 5:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return (any(t in ('Sea Beast', 'Aqua', 'Sea Animal') for t in traits)
                        or any('TS' in t for t in traits))

            def on_tuck(selected):
                if selected is None:
                    return
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    perm.card_sources.insert(0, selected)
                # Only freeze opponent perm if card was successfully placed (matches C# guard)
                if selected not in perm.card_sources:
                    return
                def on_target(target):
                    from digimon_gym.engine.interfaces.modifiers import ModifierType
                    game.register_modifier(
                        ModifierType.CANNOT_SUSPEND, target,
                        value_fn=lambda: True, expiry='end_of_opponent_turn')
                game.effect_select_opponent_permanent(
                    player, on_target,
                    filter_fn=lambda p: p.is_digimon or p.is_tamer,
                    is_optional=False,
                    prompt="Select opponent's Digimon/Tamer that can't suspend.")

            game.effect_select_hand_card(
                player, tuck_filter, on_tuck, is_optional=True,
                prompt="Select a card to place as bottom digivolution card.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing 1 play cost 5 or lower card with the [Sea Beast] or [TS] trait or [Aqua] or [Sea Animal] in any of its traits from your hand as this Digimon's bottom digivolution card, 1 of your opponent's Digimon or Tamers can't suspend until their turn ends.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-029 By placing 1 5 or lower play cost [TS]/[Sea Beast]/[Aqua]/[Sea Animal] card as bottom source, 1 digimon/tamer cant suspend")
        effect2.set_effect_description("[When Digivolving] By placing 1 play cost 5 or lower card with the [Sea Beast] or [TS] trait or [Aqua] or [Sea Animal] in any of its traits from your hand as this Digimon's bottom digivolution card, 1 of your opponent's Digimon or Tamers can't suspend until their turn ends.")
        effect2.is_when_digivolving = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            def _tuck_ok(c):
                cost = getattr(c, 'play_cost', None)
                if cost is None or cost > 5:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return (any(t in ('Sea Beast', 'Aqua', 'Sea Animal') for t in traits)
                        or any('TS' in t for t in traits))
            return any(_tuck_ok(c) for c in player.hand_cards)

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """By tucking a card from hand, freeze 1 opponent perm."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def tuck_filter(c):
                cost = getattr(c, 'play_cost', None)
                if cost is None or cost > 5:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return (any(t in ('Sea Beast', 'Aqua', 'Sea Animal') for t in traits)
                        or any('TS' in t for t in traits))

            def on_tuck(selected):
                if selected is None:
                    return
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    perm.card_sources.insert(0, selected)
                # Only freeze opponent perm if card was successfully placed
                if selected not in perm.card_sources:
                    return
                def on_target(target):
                    from digimon_gym.engine.interfaces.modifiers import ModifierType
                    game.register_modifier(
                        ModifierType.CANNOT_SUSPEND, target,
                        value_fn=lambda: True, expiry='end_of_opponent_turn')
                game.effect_select_opponent_permanent(
                    player, on_target,
                    filter_fn=lambda p: p.is_digimon or p.is_tamer,
                    is_optional=False,
                    prompt="Select opponent's Digimon/Tamer that can't suspend.")

            game.effect_select_hand_card(
                player, tuck_filter, on_tuck, is_optional=True,
                prompt="Select a card to place as bottom digivolution card.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack] [Once Per Turn] You may play 1 play cost 5 or lower [TS] trait card from this Digimon's digivolution cards without paying the cost.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndAttack)
        effect3.set_effect_name("BT24-029 Play 1 5 cost or lower [TS] card from digivolution sources")
        effect3.set_effect_description("[End of Attack] [Once Per Turn] You may play 1 play cost 5 or lower [TS] trait card from this Digimon's digivolution cards without paying the cost.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT24_029_EOA")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return False
            top = perm.top_card
            for cs in perm.card_sources:
                if cs is top:
                    continue
                cost = getattr(cs, 'play_cost', None)
                if cost is None or cost > 5:
                    continue
                traits = getattr(cs, 'card_traits', []) or []
                if any('TS' in t for t in traits):
                    return True
            return False

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Play cost 5 or lower [TS] card from digi sources."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            top = perm.top_card
            eligible = []
            for cs in perm.card_sources:
                if cs is top:
                    continue
                cost = getattr(cs, 'play_cost', None)
                if cost is None or cost > 5:
                    continue
                traits = getattr(cs, 'card_traits', []) or []
                if not any('TS' in t for t in traits):
                    continue
                eligible.append(cs)
            if not eligible:
                return
            selected = eligible[0]
            perm.card_sources.remove(selected)
            played = player.play_card_from_source(selected, pay_cost=False)
            game.execute_effects(
                EffectTiming.OnEnterFieldAnyone,
                {"played_card": selected, "played_permanent": played,
                 "event_player": player},
            )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] You may play 1 level 4 or lower blue Digimon card with the [TS] trait from this Digimon's digivolution cards without paying the cost.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("BT24-029 Play 1 level 4 or lower blue [TS] digimon in digivolution sources")
        effect4.set_effect_description("[When Attacking] [Once Per Turn] You may play 1 level 4 or lower blue Digimon card with the [TS] trait from this Digimon's digivolution cards without paying the cost.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_029_ESS")
        effect4.is_on_attack = True

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
                if not getattr(cs, 'is_digimon', False):
                    continue
                level = getattr(cs, 'level', None)
                if level is None or level > 4:
                    continue
                colors = [c.name for c in (getattr(cs, 'card_colors', None) or [])]
                if 'Blue' not in colors:
                    continue
                traits = getattr(cs, 'card_traits', []) or []
                if any('TS' in t for t in traits):
                    return True
            return False

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Play Lv.4 or lower blue [TS] Digimon from digi sources."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            top = perm.top_card
            eligible = []
            for cs in perm.card_sources:
                if cs is top:
                    continue
                if not getattr(cs, 'is_digimon', False):
                    continue
                level = getattr(cs, 'level', None)
                if level is None or level > 4:
                    continue
                colors = [c.name for c in (getattr(cs, 'card_colors', None) or [])]
                if 'Blue' not in colors:
                    continue
                traits = getattr(cs, 'card_traits', []) or []
                if not any('TS' in t for t in traits):
                    continue
                eligible.append(cs)
            if not eligible:
                return
            selected = eligible[0]
            perm.card_sources.remove(selected)
            played = player.play_card_from_source(selected, pay_cost=False)
            game.execute_effects(
                EffectTiming.OnEnterFieldAnyone,
                {"played_card": selected, "played_permanent": played,
                 "event_player": player},
            )

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
