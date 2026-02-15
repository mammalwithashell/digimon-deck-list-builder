from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_027(CardScript):
    """BT20-027 Slayerdramon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-027 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Wingdramon] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Wingdramon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Wingdramon') or permanent.contains_card_name('Groundramon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash any 3 digivolution cards of 1 of your opponent's Digimon. Then, delete 1 of their Digimon with no digivolution cards.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-027 Trash digivolution cards and delete 1 Digimon with no digivolution cards")
        effect1.set_effect_description("[On Play] Trash any 3 digivolution cards of 1 of your opponent's Digimon. Then, delete 1 of their Digimon with no digivolution cards.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete, Trash Digivolution Cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash any 3 digivolution cards of 1 of your opponent's Digimon. Then, delete 1 of their Digimon with no digivolution cards.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-027 Trash digivolution cards and delete 1 Digimon with no digivolution cards")
        effect2.set_effect_description("[When Digivolving] Trash any 3 digivolution cards of 1 of your opponent's Digimon. Then, delete 1 of their Digimon with no digivolution cards.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete, Trash Digivolution Cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] (Once Per Turn) When your opponent's security stack is removed from, 1 of your Digimon with [Dracomon]/[Examon] in its text may unsuspend.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-027 Unsuspend 1 of your Digimon with [Dracomon]/[Examon] in its text")
        effect3.set_effect_description("[All Turns] (Once Per Turn) When your opponent's security stack is removed from, 1 of your Digimon with [Dracomon]/[Examon] in its text may unsuspend.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Unsuspend_BT20_027")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Dracomon' in text or 'Examon' in text):
                    return False
            else:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When any of your Digimon with [Dracomon]/[Examon] in their texts would leave the battle area other than in battle by suspending this Digimon, they don't leave.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-027 Suspend this Digimon to prevent Digimon from leaving")
        effect4.set_effect_description("[All Turns] When any of your Digimon with [Dracomon]/[Examon] in their texts would leave the battle area other than in battle by suspending this Digimon, they don't leave.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("AllTurns_BT20-027")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Dracomon' in text or 'Examon' in text):
                    return False
            else:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
