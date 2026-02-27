from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_007(CardScript):
    """BT22-007 Mother Eater"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Breeding] [Start of Your Main Phase] Look at your Digi-Egg deck's top card. Among them, you may place [Mother Eater]s as this Digimon's top digivolution cards. Then, if this Digimon has 10 or more digivolution cards, you may play 3 [Mother Eater]s from its digivolution cards without paying the costs.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-007 Add [Mother Eater]s to top of stack. if 10 or more in digivolution source, play 3 [Mother Eater]s")
        effect0.set_effect_description("[Breeding] [Start of Your Main Phase] Look at your Digi-Egg deck's top card. Among them, you may place [Mother Eater]s as this Digimon's top digivolution cards. Then, if this Digimon has 10 or more digivolution cards, you may play 3 [Mother Eater]s from its digivolution cards without paying the costs.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 10):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-007 Delete 1 Digimon")
        effect1.set_effect_description("[On Play] Delete 1 of your opponent's Digimon.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete"""
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [Breeding] [All Turns] [Once Per Turn] When any of your [Eater] trait Digimon would leave the battle area other than by your effects, you may place them as this Digimon's bottom digivolution cards.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-007 Place card as source")
        effect2.set_effect_description("[Breeding] [All Turns] [Once Per Turn] When any of your [Eater] trait Digimon would leave the battle area other than by your effects, you may place them as this Digimon's bottom digivolution cards.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT22_007_Save")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnRemovedField
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-007 Effect")
        effect3.set_effect_description("Effect")
        effect3.is_optional = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Mother Eater'))):
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
