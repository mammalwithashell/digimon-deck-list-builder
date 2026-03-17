from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_051(CardScript):
    """BT21-051 Puppetmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: with [WG] trait for cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-051 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_trait = "WG"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('WG' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-051 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-051 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: reboot
        effect3 = ICardEffect()
        effect3.set_effect_name("BT21-051 Reboot")
        effect3.set_effect_description("Reboot")
        effect3._is_reboot = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # --- Shared process for On Play / When Digivolving ---
        def _dedigivolve_and_bounce(ctx: Dict[str, Any]):
            """De-Digivolve 2 opponent Digimon, then return 1 suspended opponent Digimon to deck bottom."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Step 1: De-Digivolve 2 one of opponent's Digimon
            has_digimon = any(p.is_digimon for p in enemy.battle_area)
            if has_digimon:
                def on_de_digivolve(target_perm):
                    removed = target_perm.de_digivolve(2)
                    enemy.trash_cards.extend(removed)
                    _do_bounce()

                game.effect_select_opponent_permanent(
                    player, on_de_digivolve,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False)
            else:
                _do_bounce()

            def _do_bounce():
                # Step 2: Return 1 of their SUSPENDED Digimon to bottom of deck
                def suspended_digimon_filter(p):
                    return p.is_digimon and p.is_suspended

                has_target = any(suspended_digimon_filter(p) for p in enemy.battle_area)
                if not has_target:
                    return

                def on_bounce(target_perm):
                    enemy.return_permanent_to_deck_bottom(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_bounce,
                    filter_fn=suspended_digimon_filter,
                    is_optional=False)

        # [On Play] De-Digivolve 2, then return 1 suspended Digimon to deck bottom
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT21-051 <De-Digivolve 2> to 1 Digimon")
        effect4.set_effect_description("[On Play] <De-Digivolve 2> 1 of your opponent's Digimon. Then, return 1 of their suspended Digimon to the bottom of the deck.")
        effect4.is_on_play = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_dedigivolve_and_bounce)
        effects.append(effect4)

        # [When Digivolving] same effect
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("BT21-051 <De-Digivolve 2> to 1 Digimon")
        effect5.set_effect_description("[When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon. Then, return 1 of their suspended Digimon to the bottom of the deck.")
        effect5.is_when_digivolving = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect5.set_can_use_condition(condition5)
        effect5.set_on_process_callback(_dedigivolve_and_bounce)
        effects.append(effect5)

        # Ace Overflow <-4> — engine handles via card data, descriptive effect
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("BT21-051 Ace Overflow <-4>")
        effect_ace.set_effect_description("Ace Overflow <-4>")
        effect_ace.is_inherited_effect = True

        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
