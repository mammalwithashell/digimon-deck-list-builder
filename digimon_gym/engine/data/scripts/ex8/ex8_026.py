from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_026(CardScript):
    """EX8-026 MetalSeadramon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: with [DS] trait for cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-026 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_trait = "DS"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('DS' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("EX8-026 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared process for On Play / When Digivolving ---
        def _dedigivolve_and_bounce(ctx: Dict[str, Any]):
            """De-Digivolve 1 opponent Digimon, then return 1 opponent Digimon with play cost <= 7 to bottom of deck."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Step 1: De-Digivolve 1 opponent's Digimon
            has_digimon = any(p.is_digimon for p in enemy.battle_area)
            if has_digimon:
                def on_de_digivolve(target_perm):
                    removed = target_perm.de_digivolve(1)
                    enemy.trash_cards.extend(removed)
                    _do_bounce()

                game.effect_select_opponent_permanent(
                    player, on_de_digivolve,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False)
            else:
                _do_bounce()

            def _do_bounce():
                # Step 2: Return 1 opponent Digimon with play cost <= 7 to bottom of deck
                def bounce_filter(p):
                    return (p.is_digimon and
                            hasattr(p.top_card, 'get_cost_itself') and
                            p.top_card.get_cost_itself <= 7)

                has_target = any(bounce_filter(p) for p in enemy.battle_area)
                if not has_target:
                    return

                def on_bounce(target_perm):
                    enemy.return_permanent_to_deck_bottom(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_bounce, filter_fn=bounce_filter, is_optional=False)

        # [On Play] De-Digivolve 1, then bounce play cost 7 or less to deck bottom
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX8-026 <De-Digivolve 1>, Return 1 Digimon with 7 cost or less to bottom of deck")
        effect2.set_effect_description("[On Play] [When Digivolving] <De-Digivolve 1> 1 of your opponent's Digimon. Then, return 1 of your opponent's Digimon with a play cost of 7 or less to the bottom of the deck.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_dedigivolve_and_bounce)
        effects.append(effect2)

        # [When Digivolving] same effect
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX8-026 <De-Digivolve 1>, Return 1 Digimon with 7 cost or less to bottom of deck")
        effect3.set_effect_description("[When Digivolving] <De-Digivolve 1> 1 of your opponent's Digimon. Then, return 1 of your opponent's Digimon with a play cost of 7 or less to the bottom of the deck.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_dedigivolve_and_bounce)
        effects.append(effect3)

        # [All Turns] While you have 1 or more memory, none of your opponent's
        # Digimon can suspend.
        # Continuous aura: register a permanent CANNOT_SUSPEND modifier with a
        # dynamic condition that checks owner's memory >= 1.
        # Registered on play / when digivolving; cleaned up when leaving field.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX8-026 Opponents Digimon Can't Suspend")
        effect4.set_effect_description("[All Turns] While you have 1 or more memory, none of your opponent's Digimon can suspend.")
        effect4.is_on_play = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Register permanent CANNOT_SUSPEND modifier on all opponent Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType

            # Condition: active when owner has 1+ memory AND target is
            # an opponent's Digimon (not our own)
            def cant_suspend_condition(target, context, _player=player, _game=game):
                # Check owner still has 1+ memory
                if _player is _game.player1:
                    if _game.memory < 1:
                        return False
                else:
                    if _game.memory > -1:
                        return False
                # Check target is opponent's Digimon
                if not target.is_digimon:
                    return False
                if target in _player.battle_area:
                    return False
                return True

            game.register_modifier(
                my_perm, ModifierType.CANNOT_SUSPEND,
                condition=cant_suspend_condition,
                expiry='permanent')

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Duplicate for [When Digivolving] timing
        effect4b = ICardEffect()
        effect4b.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4b.set_effect_name("EX8-026 Opponents Digimon Can't Suspend")
        effect4b.set_effect_description("[All Turns] While you have 1 or more memory, none of your opponent's Digimon can suspend.")
        effect4b.is_when_digivolving = True

        def condition4b(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4b.set_can_use_condition(condition4b)
        effect4b.set_on_process_callback(process4)
        effects.append(effect4b)

        # Ace Overflow <-4> — engine handles via card data, descriptive effect
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("EX8-026 Ace Overflow <-4>")
        effect_ace.set_effect_description("Ace Overflow <-4>")
        effect_ace.is_inherited_effect = True

        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
