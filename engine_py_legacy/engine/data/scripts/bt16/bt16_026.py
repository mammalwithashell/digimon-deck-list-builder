from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_026(CardScript):
    """BT16-026 Vikemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: from [Shakkoumon] or [Zudomon] for cost 3
        effect0a = ICardEffect()
        effect0a.set_effect_name("BT16-026 Alternate digivolution requirement (Shakkoumon)")
        effect0a.set_effect_description("Alternate digivolution requirement")
        effect0a._alt_digi_cost = 3
        effect0a._alt_digi_name = "Shakkoumon"

        def condition0a(context: Dict[str, Any]) -> bool:
            return True
        effect0a.set_can_use_condition(condition0a)
        effects.append(effect0a)

        effect0b = ICardEffect()
        effect0b.set_effect_name("BT16-026 Alternate digivolution requirement (Zudomon)")
        effect0b.set_effect_description("Alternate digivolution requirement")
        effect0b._alt_digi_cost = 3
        effect0b._alt_digi_name = "Zudomon"

        def condition0b(context: Dict[str, Any]) -> bool:
            return True
        effect0b.set_can_use_condition(condition0b)
        effects.append(effect0b)

        # Factory effect: blast_digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-026 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared process for On Play / When Digivolving ---
        def _dedigivolve_and_cant_suspend(ctx: Dict[str, Any]):
            """De-Digivolve 2 opponent Digimon, then apply CANNOT_SUSPEND to opponent Digimon with <= 1 digi-cards."""
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
                    _apply_cant_suspend()

                game.effect_select_opponent_permanent(
                    player, on_de_digivolve,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False)
            else:
                _apply_cant_suspend()

            def _apply_cant_suspend():
                # Step 2: All opponent's Digimon with 1 or fewer digivolution cards
                # cannot suspend until end of their turn.
                # DCGO DigivolutionCards = cards below top, so Count <= 1 means
                # card_sources length <= 2 (top + at most 1 under).
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                for p in enemy.battle_area:
                    if p.is_digimon and len(p.card_sources) - 1 <= 1:
                        game.register_modifier(
                            p, ModifierType.CANNOT_SUSPEND,
                            condition=lambda target, ctx, fp=p: target is fp,
                            expiry='end_of_opponent_turn')

        # [On Play] De-Digivolve 2, then can't suspend opponent's Digimon with <= 1 digi-cards
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT16-026 <De-Digivolve 2> 1 of your opponent's Digimon.")
        effect2.set_effect_description("[On Play] <De-Digivolve 2> 1 of your opponent's Digimon. Then, none of your opponent's Digimon with 1 or fewer digivolution cards can suspend until the end of their turn.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_dedigivolve_and_cant_suspend)
        effects.append(effect2)

        # [When Digivolving] same effect
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT16-026 <De-Digivolve 2> 1 of your opponent's Digimon.")
        effect3.set_effect_description("[When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon. Then, none of your opponent's Digimon with 1 or fewer digivolution cards can suspend until the end of their turn.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_dedigivolve_and_cant_suspend)
        effects.append(effect3)

        # [When Attacking] Delete 1 of opponent's Digimon with 1 or fewer digivolution cards.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("BT16-026 Delete 1 of your opponent's Digimon")
        effect4.set_effect_description("[When Attacking] Delete 1 of your opponent's Digimon with 1 or fewer digivolution cards.")
        effect4.is_on_attack = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Delete 1 opponent Digimon with <= 1 digivolution cards."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def target_filter(p):
                # DCGO DigivolutionCards = cards below top, Count <= 1 means
                # card_sources length <= 2 (top + at most 1 under).
                return p.is_digimon and len(p.card_sources) - 1 <= 1

            has_target = any(target_filter(p) for p in enemy.battle_area)
            if not has_target:
                return

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Ace Overflow <-4> — engine handles via card data, descriptive effect
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("BT16-026 Ace Overflow <-4>")
        effect_ace.set_effect_description("Ace Overflow <-4>")
        effect_ace.is_inherited_effect = True

        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
